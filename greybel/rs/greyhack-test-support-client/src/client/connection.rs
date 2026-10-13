use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::thread;

use duplex_core::{Channel, ChannelOptions, ErrorCode};
use duplex_driver_stdio::StreamDriver;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio::task::LocalSet;

use super::request::{Request, RequestKind};
use crate::protocol::TestClientNotification;
use crate::protocol::test_support::{
    AdvanceClockRpc, AllowFirewallRpc, BuyExploitRpc, ConfigZeroDayInfoRpc, ConnectNetworkRpc,
    CreateNetworkRpc, DisableNetworkRpc, ExecuteInteractiveRpc, ExecutePairRpc, ExecuteRpc,
    ExecuteUntilInputRpc, FindNetworkRpc, FindNewNetworkRpc, FireNpcRpc, ForceAdminLoginRpc,
    ForwardServiceRpc, FreezeClockRpc, HireRentalAdminRpc, InstallLibraryRpc, LockApplianceRpc,
    PrimeFloodRpc, ProceduralMissionRpc, PromoteAdminRpc, QueryExploitsRpc, QueryFileRpc,
    QueryGuildsRpc, QueryNetworkRpc, QueryPlayerRpc, QueryTracesRpc, QueryWordListsRpc,
    RebootPlayerRpc, RefreshNpcAccountRpc, RentNetworkRpc, RentalAdminServiceRpc, Reply,
    ReportLibraryUpdateRpc, ResetTrafficCooldownRpc, RunPaymentsRpc, SetPhoneAlarmRpc,
    SetPlayerMoneyRpc, SetRouterKernelRequirementRpc, SetupBlockchainRpc, SetupCtfRpc,
    SetupDebugRpc, SetupForeignCoinRpc, SetupGuildRpc, SetupMailRpc, SetupPlayerRpc,
    SetupTrafficRpc, SetupWalletRpc, SpawnNetworkRpc, TestClient as TestProtocol,
    TrafficAccidentRpc, TypeInputRpc, WriteFileRpc, WriteRouterKernelRpc,
};
use crate::{TestClientError, TestClientOptions};

/// Pending requests indexed by the ID repeated in their replies.
type ReplyTable = Rc<RefCell<HashMap<u32, oneshot::Sender<Result<Reply, TestClientError>>>>>;

#[derive(Clone)]
pub(super) struct Connection {
    requests: mpsc::UnboundedSender<Request>,
    timeout: std::time::Duration,
}

impl Connection {
    pub async fn connect(options: TestClientOptions) -> Result<Self, TestClientError> {
        let (ready, acknowledgement) = oneshot::channel();
        let (requests, deliveries) = mpsc::unbounded_channel();
        let timeout = options.request_timeout;

        thread::Builder::new()
            .name("greyhack test support".to_owned())
            .spawn(move || {
                let replies = ReplyTable::default();
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                let Ok(runtime) = runtime else {
                    let _ = ready.send(Err(TestClientError::Disconnect));

                    return;
                };

                LocalSet::new().block_on(&runtime, Self::run(options, deliveries, ready, replies));
            })?;
        acknowledgement
            .await
            .map_err(|_| TestClientError::Disconnect)??;

        Ok(Self { requests, timeout })
    }

    pub async fn request(&self, kind: RequestKind) -> Result<Reply, TestClientError> {
        let (response, result) = oneshot::channel();

        self.requests
            .send(Request { kind, response })
            .map_err(|_| TestClientError::Disconnect)?;

        match tokio::time::timeout(self.timeout, result).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(TestClientError::Disconnect),
            Err(_) => Err(TestClientError::Timeout(self.timeout)),
        }
    }

    async fn run(
        options: TestClientOptions,
        mut deliveries: mpsc::UnboundedReceiver<Request>,
        ready: oneshot::Sender<Result<(), TestClientError>>,
        replies: ReplyTable,
    ) {
        let address = format!("{}:{}", options.host, options.port);
        let stream = match TcpStream::connect(&address).await {
            Ok(stream) => stream,
            Err(source) => {
                let _ = ready.send(Err(TestClientError::Connect {
                    addr: address,
                    source,
                }));

                return;
            }
        };
        let transport = match StreamDriver::tcp(stream) {
            Ok(transport) => transport,
            Err(error) => {
                let _ = ready.send(Err(TestClientError::Io(error)));

                return;
            }
        };
        let channel =
            Channel::<TestProtocol, StreamDriver>::new(&transport, ChannelOptions::default());

        TestClientNotification::attach(&channel, {
            let replies = Rc::clone(&replies);

            // Drop replies without a pending request, including replies that arrive after timeout.
            move |TestClientNotification::ReplyRpc(reply)| {
                if let Some(response) = replies.borrow_mut().remove(&reply.request) {
                    let result = match reply.reply {
                        Reply::TestFailure(failure) => {
                            Err(TestClientError::Bridge(failure.message.to_string()))
                        }
                        other => Ok(other),
                    };
                    let _ = response.send(result);
                }
            }
        });

        let ready = Rc::new(RefCell::new(Some(ready)));

        channel.on_ack({
            let ready = Rc::clone(&ready);

            move |_| {
                if let Some(ready) = ready.borrow_mut().take() {
                    let _ = ready.send(Ok(()));
                }
            }
        });
        channel.on_failure({
            let ready = Rc::clone(&ready);

            move |failure| {
                if let Some(ready) = ready.borrow_mut().take() {
                    let _ = ready.send(Err(TestClientError::Refusal(failure.reason())));
                }
            }
        });

        let mut last_request = 0u32;

        loop {
            tokio::select! {
                item = deliveries.recv() => match item {
                    Some(Request { kind, response }) => {
                        last_request = last_request.wrapping_add(1);
                        let request = last_request;
                        replies.borrow_mut().insert(request, response);
                        let result = match kind {
                            RequestKind::SetupPlayer => channel.send(&SetupPlayerRpc { request }),
                            RequestKind::ConnectNetwork { public_ip } => {
                                channel.send(&ConnectNetworkRpc {
                                    request,
                                    public_ip: public_ip.into(),
                                })
                            }
                            RequestKind::WriteFile { path, content } => {
                                channel.send(&WriteFileRpc {
                                    request,
                                    path: path.into(),
                                    content: content.into(),
                                })
                            }
                            RequestKind::CreateNetwork { public_ip } => {
                                channel.send(&CreateNetworkRpc {
                                    request,
                                    public_ip: public_ip.into(),
                                })
                            }
                            RequestKind::SetupDebug => channel.send(&SetupDebugRpc { request }),
                            RequestKind::SetupCtf { event_name } => {
                                channel.send(&SetupCtfRpc { request, event_name: event_name.into() })
                            }
                            RequestKind::Execute { source, filepath, basepath, program_name } => {
                                channel.send(&ExecuteRpc {
                                    request,
                                    source: source.into(),
                                    filepath: filepath.into(),
                                    basepath: basepath.into(),
                                    program_name: program_name.into(),
                                })
                            }
                            RequestKind::ExecuteInteractive { source, filepath, basepath, program_name, inputs } => {
                                channel.send(&ExecuteInteractiveRpc {
                                    request,
                                    source: source.into(),
                                    filepath: filepath.into(),
                                    basepath: basepath.into(),
                                    program_name: program_name.into(),
                                    inputs: inputs.into(),
                                })
                            }
                            RequestKind::ExecuteUntilInput { source, filepath, basepath, program_name } => {
                                channel.send(&ExecuteUntilInputRpc {
                                    request,
                                    source: source.into(),
                                    filepath: filepath.into(),
                                    basepath: basepath.into(),
                                    program_name: program_name.into(),
                                })
                            }
                            RequestKind::TypeInput { input } => {
                                channel.send(&TypeInputRpc { request, input: input.into() })
                            }
                            RequestKind::ExecutePair { first_source, second_source, filepath, basepath, program_name } => {
                                channel.send(&ExecutePairRpc {
                                    request,
                                    first_source: first_source.into(),
                                    second_source: second_source.into(),
                                    filepath: filepath.into(),
                                    basepath: basepath.into(),
                                    program_name: program_name.into(),
                                })
                            }
                            RequestKind::QueryPlayer => channel.send(&QueryPlayerRpc { request }),
                            RequestKind::QueryFile { path } => channel.send(&QueryFileRpc { request, path: path.into() }),
                            RequestKind::SetRouterKernelRequirement { local_ip, requirement } => {
                                channel.send(&SetRouterKernelRequirementRpc {
                                    request,
                                    local_ip: local_ip.into(),
                                    requirement,
                                })
                            }
                            RequestKind::WriteRouterKernel { local_ip, content } => {
                                channel.send(&WriteRouterKernelRpc {
                                    request,
                                    local_ip: local_ip.into(),
                                    content: content.into(),
                                })
                            }
                            RequestKind::QueryNetwork => channel.send(&QueryNetworkRpc { request }),
                            RequestKind::InstallLibrary { local_ip, name, reward } => {
                                channel.send(&InstallLibraryRpc {
                                    request,
                                    local_ip: local_ip.into(),
                                    name: name.into(),
                                    reward,
                                })
                            }
                            RequestKind::SetupBlockchain { coin_name, wallet_user } => {
                                channel.send(&SetupBlockchainRpc {
                                    request,
                                    coin_name: coin_name.into(),
                                    wallet_user: wallet_user.into(),
                                })
                            }
                            RequestKind::SetupMail { user, password } => {
                                channel.send(&SetupMailRpc {
                                    request,
                                    user: user.into(),
                                    password: password.into(),
                                })
                            }
                            RequestKind::SetupWallet { user, password } => {
                                channel.send(&SetupWalletRpc {
                                    request,
                                    user: user.into(),
                                    password: password.into(),
                                })
                            }
                            RequestKind::SetupForeignCoin { coin_name, user, password } => {
                                channel.send(&SetupForeignCoinRpc {
                                    request,
                                    coin_name: coin_name.into(),
                                    user: user.into(),
                                    password: password.into(),
                                })
                            }
                            RequestKind::ForwardService { local_ip, public_port, internal_port, service } => {
                                channel.send(&ForwardServiceRpc {
                                    request,
                                    local_ip: local_ip.into(),
                                    public_port,
                                    internal_port,
                                    service: service.into(),
                                })
                            }
                            RequestKind::LockAppliance { local_ip } => {
                                channel.send(&LockApplianceRpc { request, local_ip: local_ip.into() })
                            }
                            RequestKind::PrimeFlood { local_ip } => {
                                channel.send(&PrimeFloodRpc { request, local_ip: local_ip.into() })
                            }
                            RequestKind::FreezeClock => channel.send(&FreezeClockRpc { request }),
                            RequestKind::FindNetwork { kind, device } => {
                                channel.send(&FindNetworkRpc { request, kind: kind.into(), device: device.into() })
                            }
                            RequestKind::FindNewNetwork { kind, service } => {
                                channel.send(&FindNewNetworkRpc { request, kind: kind.into(), service: service.into() })
                            }
                            RequestKind::DisableNetwork { public_ip, disabled } => {
                                channel.send(&DisableNetworkRpc { request, public_ip: public_ip.into(), disabled })
                            }
                            RequestKind::PromoteAdmin { public_ip } => {
                                channel.send(&PromoteAdminRpc { request, public_ip: public_ip.into() })
                            }
                            RequestKind::FireNpc { public_ip, user } => {
                                channel.send(&FireNpcRpc { request, public_ip: public_ip.into(), user: user.into() })
                            }
                            RequestKind::RefreshNpcAccount { public_ip, user } => {
                                channel.send(&RefreshNpcAccountRpc { request, public_ip: public_ip.into(), user: user.into() })
                            }
                            RequestKind::ConfigZeroDayInfo { public_ip } => {
                                channel.send(&ConfigZeroDayInfoRpc { request, public_ip: public_ip.into() })
                            }
                            RequestKind::SetPhoneAlarm { public_ip, user, hour, minute, delete_alarm } => {
                                channel.send(&SetPhoneAlarmRpc {
                                    request,
                                    public_ip: public_ip.into(),
                                    user: user.into(),
                                    hour,
                                    minute,
                                    delete_alarm,
                                })
                            }
                            RequestKind::SetupTraffic => channel.send(&SetupTrafficRpc { request }),
                            RequestKind::TrafficAccident { user, public_ip, police_ip } => {
                                channel.send(&TrafficAccidentRpc {
                                    request,
                                    user: user.into(),
                                    public_ip: public_ip.into(),
                                    police_ip: police_ip.into(),
                                })
                            }
                            RequestKind::ResetTrafficCooldown => channel.send(&ResetTrafficCooldownRpc { request }),
                            RequestKind::HireRentalAdmin { public_ip } => {
                                channel.send(&HireRentalAdminRpc { request, public_ip: public_ip.into() })
                            }
                            RequestKind::SpawnNetwork { kind } => {
                                channel.send(&SpawnNetworkRpc { request, kind: kind.into() })
                            }
                            RequestKind::AllowFirewall { local_ip } => {
                                channel.send(&AllowFirewallRpc { request, local_ip: local_ip.into() })
                            }
                            RequestKind::ForceAdminLogin { public_ip } => {
                                channel.send(&ForceAdminLoginRpc { request, public_ip: public_ip.into() })
                            }
                            RequestKind::AdvanceClock { seconds } => {
                                channel.send(&AdvanceClockRpc { request, seconds })
                            }
                            RequestKind::ProceduralMission { police, index } => {
                                channel.send(&ProceduralMissionRpc { request, police, index })
                            }
                            RequestKind::QueryExploits { library, version } => {
                                channel.send(&QueryExploitsRpc {
                                    request,
                                    library: library.into(),
                                    version: version.into(),
                                })
                            }
                            RequestKind::BuyExploit { library, version, exploit, folder, compile } => {
                                channel.send(&BuyExploitRpc {
                                    request,
                                    library: library.into(),
                                    version: version.into(),
                                    exploit: exploit.into(),
                                    folder: folder.into(),
                                    compile,
                                })
                            }
                            RequestKind::RentNetwork { public_ip, amount } => {
                                channel.send(&RentNetworkRpc { request, public_ip: public_ip.into(), amount })
                            }
                            RequestKind::RentalAdminService { public_ip, hire } => {
                                channel.send(&RentalAdminServiceRpc { request, public_ip: public_ip.into(), hire })
                            }
                            RequestKind::RunPayments => channel.send(&RunPaymentsRpc { request }),
                            RequestKind::SetPlayerMoney { money } => {
                                channel.send(&SetPlayerMoneyRpc { request, money })
                            }
                            RequestKind::RebootPlayer { safe_mode } => {
                                channel.send(&RebootPlayerRpc { request, safe_mode })
                            }
                            RequestKind::ReportLibraryUpdate { public_ip, library } => {
                                channel.send(&ReportLibraryUpdateRpc {
                                    request,
                                    public_ip: public_ip.into(),
                                    library: library.into(),
                                })
                            }
                            RequestKind::SetupGuild(guild) => channel.send(&SetupGuildRpc {
                                request,
                                name: guild.name.into(),
                                bank_account: guild.bank_account.into(),
                                total_exp: guild.total_exp,
                                investment_perks: guild.investment_perks,
                                join: guild.join,
                                company_ip: guild.company_ip.into(),
                                crown: guild.crown,
                                shield_ip: guild.shield_ip.into(),
                                admin_restore_perks: guild.admin_restore_perks,
                                admin_restore_off: guild.admin_restore_off,
                            }),
                            RequestKind::QueryTraces => channel.send(&QueryTracesRpc { request }),
            RequestKind::QueryWordLists => channel.send(&QueryWordListsRpc { request }),
                            RequestKind::QueryGuilds { company_ips } => {
                                channel.send(&QueryGuildsRpc {
                                    request,
                                    company_ips: company_ips.iter().map(|ip| ip.as_str().into()).collect(),
                                })
                            }
                        };
                        if let Err(error) = result
                            && let Some(response) = replies.borrow_mut().remove(&request)
                        {
                            let _ = response.send(Err(error.into()));
                        }
                    }
                    None => {
                        channel.close(ErrorCode::ClosedChannel);
                        break;
                    }
                },
                _ = channel.close_event() => break,
            }
        }

        for (_, response) in replies.borrow_mut().drain() {
            let _ = response.send(Err(TestClientError::Disconnect));
        }

        if let Some(ready) = ready.borrow_mut().take() {
            let _ = ready.send(Err(TestClientError::Disconnect));
        }
    }
}
