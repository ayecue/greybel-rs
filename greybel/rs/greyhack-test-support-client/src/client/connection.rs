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
    AdvanceClockRpc, AllowFirewallRpc, BuyExploitRpc, ConfigZeroDayInfoRpc, ConfigureApplianceRpc,
    ConfigureComputerRpc, ConfigureEconomyRpc, ConfigureMailRpc, ConnectNetworkRpc,
    CreateFixtureNetworkRpc, CreateNetworkRpc, DeleteCtfRpc, DisableNetworkRpc, EndGameRpc,
    ExecuteInTerminalRpc, ExecuteInteractiveRpc, ExecutePairRpc, ExecuteRpc, ExecuteUntilInputRpc,
    FireNpcRpc, ForceAdminLoginRpc, ForwardServiceRpc, FreezeClockRpc, HireRentalAdminRpc,
    InstallFixtureLibraryRpc, InstallLibraryRpc, LockApplianceRpc, MissionRpc, PrimeFloodRpc,
    PromoteAdminRpc, QueryExploitsRpc, QueryFileRpc, QueryGuildsRpc, QueryNetworkRpc,
    QueryPlayerRpc, QueryTracesRpc, QueryWordListsRpc, RateCtfRpc, RebootPlayerRpc,
    RefreshNpcAccountRpc, ReleaseLibraryVersionRpc, RentNetworkRpc, RentalAdminServiceRpc, Reply,
    ReportLibraryUpdateRpc, ResetTrafficCooldownRpc, RunPaymentRpc, RunPaymentsRpc,
    SetPhoneAlarmRpc, SetPlayerMoneyRpc, SetRouterKernelRequirementRpc, SetupBlockchainRpc,
    SetupCtfRpc, SetupDebugRpc, SetupForeignCoinRpc, SetupGuildRpc, SetupMailRpc, SetupPlayerRpc,
    SetupTrafficRpc, SetupWalletRpc, SpawnCtfNetworkRpc, SpawnNetworkRpc,
    TestClient as TestProtocol, TrafficAccidentRpc, TypeInputRpc, WriteFileRpc,
    WriteRouterKernelRpc,
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
                            Err(TestClientError::Bridge(failure.message.clone()))
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
                        let result = match &kind {
                            RequestKind::SetupPlayer | RequestKind::ConnectNetwork { .. } | RequestKind::WriteFile { .. } | RequestKind::CreateNetwork { .. } | RequestKind::CreateFixtureNetwork { .. } | RequestKind::SetupDebug | RequestKind::SetupCtf { .. } | RequestKind::Execute { .. } | RequestKind::ExecuteInteractive { .. } | RequestKind::ExecuteUntilInput { .. } => send_bootstrap(&channel, request, kind),
                            RequestKind::TypeInput { .. } | RequestKind::ExecutePair { .. } | RequestKind::QueryPlayer | RequestKind::QueryFile { .. } | RequestKind::SetRouterKernelRequirement { .. } | RequestKind::WriteRouterKernel { .. } | RequestKind::QueryNetwork | RequestKind::InstallLibrary { .. } | RequestKind::InstallFixtureLibrary { .. } | RequestKind::SetupBlockchain { .. } | RequestKind::SetupMail { .. } | RequestKind::SetupWallet { .. } | RequestKind::ConfigureComputer { .. } | RequestKind::ConfigureEconomy { .. } | RequestKind::ConfigureMail { .. } | RequestKind::DeleteCtf { .. } | RequestKind::ConfigureAppliance { .. } => send_execution(&channel, request, kind),
                            RequestKind::SetupForeignCoin { .. } | RequestKind::ForwardService { .. } | RequestKind::LockAppliance { .. } | RequestKind::PrimeFlood { .. } | RequestKind::FreezeClock | RequestKind::DisableNetwork { .. } | RequestKind::PromoteAdmin { .. } | RequestKind::FireNpc { .. } => send_network(&channel, request, kind),
                            RequestKind::RefreshNpcAccount { .. } | RequestKind::ConfigZeroDayInfo { .. } | RequestKind::SetPhoneAlarm { .. } | RequestKind::SetupTraffic | RequestKind::TrafficAccident { .. } | RequestKind::ResetTrafficCooldown | RequestKind::HireRentalAdmin { .. } | RequestKind::SpawnNetwork { .. } | RequestKind::AllowFirewall { .. } | RequestKind::ForceAdminLogin { .. } | RequestKind::AdvanceClock { .. } => send_network_admin(&channel, request, kind),
                            RequestKind::Mission { .. } | RequestKind::QueryExploits { .. } | RequestKind::BuyExploit { .. } | RequestKind::RentNetwork { .. } | RequestKind::RentalAdminService { .. } | RequestKind::RunPayments | RequestKind::SetPlayerMoney { .. } | RequestKind::RebootPlayer { .. } | RequestKind::ReportLibraryUpdate { .. } | RequestKind::ReleaseLibraryVersion { .. } | RequestKind::SetupGuild(_) | RequestKind::QueryTraces => send_economy(&channel, request, kind),
                            RequestKind::QueryWordLists | RequestKind::RunPayment { .. } | RequestKind::EndGame { .. } | RequestKind::QueryGuilds { .. } | RequestKind::ExecuteInTerminal { .. } | RequestKind::RateCtf { .. } | RequestKind::SpawnCtfNetwork => send_player(&channel, request, kind),
                        };
                        if let Err(error) = result
                            && let Some(response) = replies.borrow_mut().remove(&request)
                        {
                            let _ = response.send(Err(error));
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

fn send_bootstrap(
    channel: &Channel<TestProtocol, StreamDriver>,
    request: u32,
    kind: RequestKind,
) -> Result<(), TestClientError> {
    match kind {
        RequestKind::SetupPlayer => channel.send(&SetupPlayerRpc { request }),
        RequestKind::ConnectNetwork { public_ip } => channel.send(&ConnectNetworkRpc {
            request,
            public_ip: public_ip.into(),
        }),
        RequestKind::WriteFile { path, content } => channel.send(&WriteFileRpc {
            request,
            path: path.into(),
            content: content.into(),
        }),
        RequestKind::CreateNetwork { public_ip } => channel.send(&CreateNetworkRpc {
            request,
            public_ip: public_ip.into(),
        }),
        RequestKind::CreateFixtureNetwork {
            kind,
            layout,
            oldest_kernel,
            targets,
        } => channel.send(&CreateFixtureNetworkRpc {
            request,
            kind: kind.into(),
            layout: layout.into(),
            oldest_kernel,
            targets: targets.into(),
        }),
        RequestKind::SetupDebug => channel.send(&SetupDebugRpc { request }),
        RequestKind::SetupCtf { event_name } => channel.send(&SetupCtfRpc {
            request,
            event_name: event_name.into(),
        }),
        RequestKind::Execute {
            source,
            filepath,
            basepath,
            program_name,
        } => channel.send(&ExecuteRpc {
            request,
            source: source.into(),
            filepath: filepath.into(),
            basepath: basepath.into(),
            program_name: program_name.into(),
        }),
        RequestKind::ExecuteInteractive {
            source,
            filepath,
            basepath,
            program_name,
            inputs,
        } => channel.send(&ExecuteInteractiveRpc {
            request,
            source: source.into(),
            filepath: filepath.into(),
            basepath: basepath.into(),
            program_name: program_name.into(),
            inputs: inputs.into(),
        }),
        RequestKind::ExecuteUntilInput {
            source,
            filepath,
            basepath,
            program_name,
        } => channel.send(&ExecuteUntilInputRpc {
            request,
            source: source.into(),
            filepath: filepath.into(),
            basepath: basepath.into(),
            program_name: program_name.into(),
        }),
        _ => unreachable!(),
    }
    .map_err(Into::into)
}

#[expect(
    clippy::too_many_lines,
    reason = "Test-support request kinds share one wire-dispatch table"
)]
fn send_execution(
    channel: &Channel<TestProtocol, StreamDriver>,
    request: u32,
    kind: RequestKind,
) -> Result<(), TestClientError> {
    match kind {
        RequestKind::TypeInput { input } => channel.send(&TypeInputRpc {
            request,
            input: input.into(),
        }),
        RequestKind::ExecutePair {
            first_source,
            second_source,
            filepath,
            basepath,
            program_name,
        } => channel.send(&ExecutePairRpc {
            request,
            first_source: first_source.into(),
            second_source: second_source.into(),
            filepath: filepath.into(),
            basepath: basepath.into(),
            program_name: program_name.into(),
        }),
        RequestKind::QueryPlayer => channel.send(&QueryPlayerRpc { request }),
        RequestKind::QueryFile { path } => channel.send(&QueryFileRpc {
            request,
            path: path.into(),
        }),
        RequestKind::SetRouterKernelRequirement {
            local_ip,
            requirement,
        } => channel.send(&SetRouterKernelRequirementRpc {
            request,
            local_ip: local_ip.into(),
            requirement,
        }),
        RequestKind::WriteRouterKernel { local_ip, content } => {
            channel.send(&WriteRouterKernelRpc {
                request,
                local_ip: local_ip.into(),
                content: content.into(),
            })
        }
        RequestKind::QueryNetwork => channel.send(&QueryNetworkRpc { request }),
        RequestKind::ConfigureComputer { local_ip, fixture } => {
            channel.send(&ConfigureComputerRpc {
                request,
                local_ip: local_ip.into(),
                fixture: fixture.into(),
            })
        }
        RequestKind::InstallLibrary {
            local_ip,
            name,
            rewards,
        } => channel.send(&InstallLibraryRpc {
            request,
            local_ip: local_ip.into(),
            name: name.into(),
            rewards: rewards.into(),
        }),
        RequestKind::InstallFixtureLibrary {
            local_ip,
            name,
            weaknesses,
            register_exploits,
        } => channel.send(&InstallFixtureLibraryRpc {
            request,
            local_ip: local_ip.into(),
            name: name.into(),
            weaknesses: weaknesses.into(),
            register_exploits,
        }),
        RequestKind::SetupBlockchain {
            coin_name,
            wallet_user,
        } => channel.send(&SetupBlockchainRpc {
            request,
            coin_name: coin_name.into(),
            wallet_user: wallet_user.into(),
        }),
        RequestKind::SetupMail { user, password } => channel.send(&SetupMailRpc {
            request,
            user: user.into(),
            password: password.into(),
        }),
        RequestKind::SetupWallet { user, password } => channel.send(&SetupWalletRpc {
            request,
            user: user.into(),
            password: password.into(),
        }),
        RequestKind::ConfigureMail {
            fixture,
            address,
            sender,
        } => channel.send(&ConfigureMailRpc {
            request,
            fixture: fixture.into(),
            address: address.into(),
            sender: sender.into(),
        }),
        RequestKind::DeleteCtf { event_name } => channel.send(&DeleteCtfRpc {
            request,
            event_name: event_name.into(),
        }),
        RequestKind::ConfigureAppliance { local_ip, fixture } => {
            channel.send(&ConfigureApplianceRpc {
                request,
                local_ip: local_ip.into(),
                fixture: fixture.into(),
            })
        }
        RequestKind::ConfigureEconomy {
            fixture,
            coin,
            wallet,
            subwallet,
        } => channel.send(&ConfigureEconomyRpc {
            request,
            fixture: fixture.into(),
            coin: coin.into(),
            wallet: wallet.into(),
            subwallet: subwallet.into(),
        }),
        _ => unreachable!(),
    }
    .map_err(Into::into)
}

fn send_network(
    channel: &Channel<TestProtocol, StreamDriver>,
    request: u32,
    kind: RequestKind,
) -> Result<(), TestClientError> {
    match kind {
        RequestKind::SetupForeignCoin {
            coin_name,
            user,
            password,
        } => channel.send(&SetupForeignCoinRpc {
            request,
            coin_name: coin_name.into(),
            user: user.into(),
            password: password.into(),
        }),
        RequestKind::ForwardService {
            local_ip,
            public_port,
            internal_port,
            service,
        } => channel.send(&ForwardServiceRpc {
            request,
            local_ip: local_ip.into(),
            public_port,
            internal_port,
            service: service.into(),
        }),
        RequestKind::LockAppliance { local_ip } => channel.send(&LockApplianceRpc {
            request,
            local_ip: local_ip.into(),
        }),
        RequestKind::PrimeFlood { local_ip } => channel.send(&PrimeFloodRpc {
            request,
            local_ip: local_ip.into(),
        }),
        RequestKind::FreezeClock => channel.send(&FreezeClockRpc { request }),
        RequestKind::DisableNetwork {
            public_ip,
            disabled,
        } => channel.send(&DisableNetworkRpc {
            request,
            public_ip: public_ip.into(),
            disabled,
        }),
        RequestKind::PromoteAdmin { public_ip } => channel.send(&PromoteAdminRpc {
            request,
            public_ip: public_ip.into(),
        }),
        RequestKind::FireNpc { public_ip, user } => channel.send(&FireNpcRpc {
            request,
            public_ip: public_ip.into(),
            user: user.into(),
        }),
        _ => unreachable!(),
    }
    .map_err(Into::into)
}

fn send_network_admin(
    channel: &Channel<TestProtocol, StreamDriver>,
    request: u32,
    kind: RequestKind,
) -> Result<(), TestClientError> {
    match kind {
        RequestKind::RefreshNpcAccount { public_ip, user } => channel.send(&RefreshNpcAccountRpc {
            request,
            public_ip: public_ip.into(),
            user: user.into(),
        }),
        RequestKind::ConfigZeroDayInfo { public_ip } => channel.send(&ConfigZeroDayInfoRpc {
            request,
            public_ip: public_ip.into(),
        }),
        RequestKind::SetPhoneAlarm {
            public_ip,
            user,
            hour,
            minute,
            delete_alarm,
        } => channel.send(&SetPhoneAlarmRpc {
            request,
            public_ip: public_ip.into(),
            user: user.into(),
            hour,
            minute,
            delete_alarm,
        }),
        RequestKind::SetupTraffic => channel.send(&SetupTrafficRpc { request }),
        RequestKind::TrafficAccident {
            user,
            public_ip,
            police_ip,
        } => channel.send(&TrafficAccidentRpc {
            request,
            user: user.into(),
            public_ip: public_ip.into(),
            police_ip: police_ip.into(),
        }),
        RequestKind::ResetTrafficCooldown => channel.send(&ResetTrafficCooldownRpc { request }),
        RequestKind::HireRentalAdmin { public_ip } => channel.send(&HireRentalAdminRpc {
            request,
            public_ip: public_ip.into(),
        }),
        RequestKind::SpawnNetwork { kind } => channel.send(&SpawnNetworkRpc {
            request,
            kind: kind.into(),
        }),
        RequestKind::AllowFirewall { local_ip } => channel.send(&AllowFirewallRpc {
            request,
            local_ip: local_ip.into(),
        }),
        RequestKind::ForceAdminLogin { public_ip } => channel.send(&ForceAdminLoginRpc {
            request,
            public_ip: public_ip.into(),
        }),
        RequestKind::AdvanceClock { seconds } => {
            channel.send(&AdvanceClockRpc { request, seconds })
        }
        _ => unreachable!(),
    }
    .map_err(Into::into)
}

fn send_economy(
    channel: &Channel<TestProtocol, StreamDriver>,
    request: u32,
    kind: RequestKind,
) -> Result<(), TestClientError> {
    match kind {
        RequestKind::Mission { kind, index } => channel.send(&MissionRpc {
            request,
            kind: kind.into(),
            index,
        }),
        RequestKind::QueryExploits { library, version } => channel.send(&QueryExploitsRpc {
            request,
            library: library.into(),
            version: version.into(),
        }),
        RequestKind::BuyExploit {
            library,
            version,
            exploit,
            folder,
            compile,
        } => channel.send(&BuyExploitRpc {
            request,
            library: library.into(),
            version: version.into(),
            exploit: exploit.into(),
            folder: folder.into(),
            compile,
        }),
        RequestKind::RentNetwork { public_ip, amount } => channel.send(&RentNetworkRpc {
            request,
            public_ip: public_ip.into(),
            amount,
        }),
        RequestKind::RentalAdminService { public_ip, hire } => {
            channel.send(&RentalAdminServiceRpc {
                request,
                public_ip: public_ip.into(),
                hire,
            })
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
        RequestKind::ReleaseLibraryVersion { library } => channel.send(&ReleaseLibraryVersionRpc {
            request,
            library: library.into(),
        }),
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
        _ => unreachable!(),
    }
    .map_err(Into::into)
}

fn send_player(
    channel: &Channel<TestProtocol, StreamDriver>,
    request: u32,
    kind: RequestKind,
) -> Result<(), TestClientError> {
    match kind {
        RequestKind::QueryWordLists => channel.send(&QueryWordListsRpc { request }),
        RequestKind::RunPayment { kind } => channel.send(&RunPaymentRpc {
            request,
            kind: kind.as_str().into(),
        }),
        RequestKind::EndGame {
            strikes,
            company_ip,
        } => channel.send(&EndGameRpc {
            request,
            strikes,
            company_ip: company_ip.as_str().into(),
        }),
        RequestKind::QueryGuilds { company_ips } => channel.send(&QueryGuildsRpc {
            request,
            company_ips: company_ips.iter().map(|ip| ip.as_str().into()).collect(),
        }),
        RequestKind::ExecuteInTerminal {
            source,
            filepath,
            basepath,
            program_name,
            safe_terminal,
        } => channel.send(&ExecuteInTerminalRpc {
            request,
            source: source.into(),
            filepath: filepath.into(),
            basepath: basepath.into(),
            program_name: program_name.into(),
            safe_terminal,
        }),
        RequestKind::RateCtf {
            event_name,
            quality,
            difficulty,
        } => channel.send(&RateCtfRpc {
            request,
            event_name: event_name.into(),
            quality,
            difficulty,
        }),
        RequestKind::SpawnCtfNetwork => channel.send(&SpawnCtfNetworkRpc { request }),
        _ => unreachable!(),
    }
    .map_err(Into::into)
}
