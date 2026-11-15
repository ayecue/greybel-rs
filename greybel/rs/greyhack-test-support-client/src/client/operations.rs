use super::{GuildSetup, RequestKind, TestClient};
use crate::TestClientError;
use crate::protocol::test_support::{
    BlockchainReadyReply, CtfRatedReply, DebugReadyReply, ExecutionResultReply, ExploitBoughtReply,
    ExploitsReply, FileStateReply, FileWrittenReply, FixtureLibraryReply, FixtureNetworkReply,
    FixtureTarget, FixtureWeakness, GameEndedReply, GuildsReply, KernelReplacedReply,
    LibraryReleasedReply, LibraryReportedReply, MissionReply, NetworkConnectionReply,
    NetworkCreatedReply, NetworkSnapshot, NetworkStateReply, PairExecutionResultReply,
    PlayerReadyReply, PlayerStateReply, Reply, TracesReply, TrafficReadyReply, WordListsReply,
};

impl TestClient {
    pub async fn setup_player(&self) -> Result<PlayerReadyReply, TestClientError> {
        match self.request(RequestKind::SetupPlayer).await? {
            Reply::PlayerReady(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("PlayerReady")),
        }
    }

    pub async fn connect_network(
        &self,
        public_ip: &str,
    ) -> Result<NetworkConnectionReply, TestClientError> {
        match self
            .request(RequestKind::ConnectNetwork {
                public_ip: public_ip.to_owned(),
            })
            .await?
        {
            Reply::NetworkConnection(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("NetworkConnection")),
        }
    }

    pub async fn write_file(
        &self,
        path: &str,
        content: &str,
    ) -> Result<FileWrittenReply, TestClientError> {
        match self
            .request(RequestKind::WriteFile {
                path: path.to_owned(),
                content: content.to_owned(),
            })
            .await?
        {
            Reply::FileWritten(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("FileWritten")),
        }
    }

    pub async fn create_network(
        &self,
        public_ip: &str,
    ) -> Result<NetworkCreatedReply, TestClientError> {
        match self
            .request(RequestKind::CreateNetwork {
                public_ip: public_ip.to_owned(),
            })
            .await?
        {
            Reply::NetworkCreated(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("NetworkCreated")),
        }
    }

    /// Creates a fresh mission-free network with the requested generated targets.
    pub async fn create_fixture_network(
        &self,
        kind: &str,
        layout: &str,
        oldest_kernel: bool,
        targets: Vec<FixtureTarget>,
    ) -> Result<FixtureNetworkReply, TestClientError> {
        match self
            .request(RequestKind::CreateFixtureNetwork {
                kind: kind.to_owned(),
                layout: layout.to_owned(),
                oldest_kernel,
                targets,
            })
            .await?
        {
            Reply::FixtureNetwork(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("FixtureNetwork")),
        }
    }

    /// Generates debug credentials for the current player through the game.
    pub async fn setup_debug(&self) -> Result<DebugReadyReply, TestClientError> {
        match self.request(RequestKind::SetupDebug).await? {
            Reply::DebugReady(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("DebugReady")),
        }
    }

    /// Uses the game's mission RPCs to create and accept a CTF event.
    pub async fn setup_ctf(&self, event_name: &str) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::SetupCtf {
            event_name: event_name.to_owned(),
        })
        .await
    }

    /// Rates a CTF event through the game's `CTF_Event.AddScore` path.
    pub async fn rate_ctf(
        &self,
        event_name: &str,
        quality: i64,
        difficulty: i64,
    ) -> Result<CtfRatedReply, TestClientError> {
        match self
            .request(RequestKind::RateCtf {
                event_name: event_name.to_owned(),
                quality,
                difficulty,
            })
            .await?
        {
            Reply::CtfRated(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("CtfRated")),
        }
    }

    pub async fn execute(
        &self,
        source: &str,
        filepath: &str,
        basepath: &str,
        program_name: &str,
    ) -> Result<ExecutionResultReply, TestClientError> {
        match self
            .request(RequestKind::Execute {
                source: source.to_owned(),
                filepath: filepath.to_owned(),
                basepath: basepath.to_owned(),
                program_name: program_name.to_owned(),
            })
            .await?
        {
            Reply::ExecutionResult(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("ExecutionResult")),
        }
    }

    /// Executes a script as typed into the safe-mode terminal of the player's computer when `safe_terminal` is set,
    /// otherwise into a terminal that isn't protected (`ExecuteInTerminalRpc`).
    pub async fn execute_in_terminal(
        &self,
        source: &str,
        filepath: &str,
        basepath: &str,
        program_name: &str,
        safe_terminal: bool,
    ) -> Result<ExecutionResultReply, TestClientError> {
        match self
            .request(RequestKind::ExecuteInTerminal {
                source: source.to_owned(),
                filepath: filepath.to_owned(),
                basepath: basepath.to_owned(),
                program_name: program_name.to_owned(),
                safe_terminal,
            })
            .await?
        {
            Reply::ExecutionResult(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("ExecutionResult")),
        }
    }

    /// Executes a script and supplies each input after it prints `marker`.
    pub async fn execute_interactive(
        &self,
        source: &str,
        filepath: &str,
        basepath: &str,
        program_name: &str,
        inputs: &[String],
    ) -> Result<ExecutionResultReply, TestClientError> {
        match self
            .request(RequestKind::ExecuteInteractive {
                source: source.to_owned(),
                filepath: filepath.to_owned(),
                basepath: basepath.to_owned(),
                program_name: program_name.to_owned(),
                inputs: inputs.to_vec(),
            })
            .await?
        {
            Reply::ExecutionResult(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("ExecutionResult")),
        }
    }

    /// Runs a script until it requests input or exits.
    pub async fn execute_until_input(
        &self,
        source: &str,
        filepath: &str,
        basepath: &str,
        program_name: &str,
    ) -> Result<ExecutionStep, TestClientError> {
        self.request(RequestKind::ExecuteUntilInput {
            source: source.to_owned(),
            filepath: filepath.to_owned(),
            basepath: basepath.to_owned(),
            program_name: program_name.to_owned(),
        })
        .await
        .and_then(ExecutionStep::of)
    }

    /// Supplies `input` to the script started by [`execute_until_input`](Self::execute_until_input), then resumes it
    /// until its next input request or exit.
    pub async fn type_input(&self, input: &str) -> Result<ExecutionStep, TestClientError> {
        self.request(RequestKind::TypeInput {
            input: input.to_owned(),
        })
        .await
        .and_then(ExecutionStep::of)
    }

    /// Runs two scripts concurrently and returns results in argument order.
    pub async fn execute_pair(
        &self,
        first_source: &str,
        second_source: &str,
        filepath: &str,
        basepath: &str,
        program_name: &str,
    ) -> Result<PairExecutionResultReply, TestClientError> {
        match self
            .request(RequestKind::ExecutePair {
                first_source: first_source.to_owned(),
                second_source: second_source.to_owned(),
                filepath: filepath.to_owned(),
                basepath: basepath.to_owned(),
                program_name: program_name.to_owned(),
            })
            .await?
        {
            Reply::PairExecutionResult(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("PairExecutionResult")),
        }
    }

    pub async fn player_state(&self) -> Result<PlayerStateReply, TestClientError> {
        match self.request(RequestKind::QueryPlayer).await? {
            Reply::PlayerState(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("PlayerState")),
        }
    }

    pub async fn file_state(&self, path: &str) -> Result<FileStateReply, TestClientError> {
        match self
            .request(RequestKind::QueryFile {
                path: path.to_owned(),
            })
            .await?
        {
            Reply::FileState(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("FileState")),
        }
    }

    /// Selects a game-generated router kernel that has the required weakness and returns its original JSON.
    pub async fn set_router_kernel_requirement(
        &self,
        local_ip: &str,
        requirement: u8,
    ) -> Result<KernelReplacedReply, TestClientError> {
        match self
            .request(RequestKind::SetRouterKernelRequirement {
                local_ip: local_ip.to_owned(),
                requirement,
            })
            .await?
        {
            Reply::KernelReplaced(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("KernelReplaced")),
        }
    }

    /// Restores the router kernel's original JSON content.
    pub async fn write_router_kernel(
        &self,
        local_ip: &str,
        content: &str,
    ) -> Result<(), TestClientError> {
        match self
            .request(RequestKind::WriteRouterKernel {
                local_ip: local_ip.to_owned(),
                content: content.to_owned(),
            })
            .await?
        {
            Reply::Applied => Ok(()),
            _ => Err(TestClientError::UnexpectedResponse("Applied")),
        }
    }

    /// Returns the player's computer and current network as reported by the game.
    pub async fn network_state(&self) -> Result<NetworkSnapshot, TestClientError> {
        match self.request(RequestKind::QueryNetwork).await? {
            Reply::NetworkState(NetworkStateReply { network }) => Ok(network),
            _ => Err(TestClientError::UnexpectedResponse("NetworkState")),
        }
    }

    /// Places a new game-generated library `name` with a free weakness for `reward` in `/lib` on `local_ip`.
    /// An empty `local_ip` selects the player's computer.
    pub async fn install_library(
        &self,
        local_ip: &str,
        name: &str,
        reward: u8,
    ) -> Result<(), TestClientError> {
        self.install_library_with(local_ip, name, &[reward]).await
    }

    /// Places a new game-generated library `name` with a requirement-free weakness for each of `rewards` in `/lib`
    /// on `local_ip` in the player's network.
    pub async fn install_library_with(
        &self,
        local_ip: &str,
        name: &str,
        rewards: &[u8],
    ) -> Result<(), TestClientError> {
        match self
            .request(RequestKind::InstallLibrary {
                local_ip: local_ip.to_owned(),
                name: name.to_owned(),
                rewards: rewards.to_vec(),
            })
            .await?
        {
            Reply::Applied => Ok(()),
            _ => Err(TestClientError::UnexpectedResponse("Applied")),
        }
    }

    /// Installs a new game-generated library version with exact weakness coverage.
    pub async fn install_fixture_library(
        &self,
        local_ip: &str,
        name: &str,
        weaknesses: Vec<FixtureWeakness>,
        register_exploits: bool,
    ) -> Result<FixtureLibraryReply, TestClientError> {
        match self
            .request(RequestKind::InstallFixtureLibrary {
                local_ip: local_ip.to_owned(),
                name: name.to_owned(),
                weaknesses,
                register_exploits,
            })
            .await?
        {
            Reply::FixtureLibrary(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("FixtureLibrary")),
        }
    }

    /// Creates the game's bank, custom-coin, and counterparty fixtures for blockchain comparisons.
    pub async fn setup_blockchain(
        &self,
        coin_name: &str,
        wallet_user: &str,
    ) -> Result<BlockchainReadyReply, TestClientError> {
        match self
            .request(RequestKind::SetupBlockchain {
                coin_name: coin_name.to_owned(),
                wallet_user: wallet_user.to_owned(),
            })
            .await?
        {
            Reply::BlockchainReady(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("BlockchainReady")),
        }
    }

    /// Registers the player's mail account with the current network's SMTP server.
    pub async fn setup_mail(&self, user: &str, password: &str) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::SetupMail {
            user: user.to_owned(),
            password: password.to_owned(),
        })
        .await
    }

    /// Creates a player-owned wallet with its initial password reset available.
    pub async fn setup_wallet(&self, user: &str, password: &str) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::SetupWallet {
            user: user.to_owned(),
            password: password.to_owned(),
        })
        .await
    }

    /// Creates a coin for the fixture counterparty.
    pub async fn setup_foreign_coin(
        &self,
        coin_name: &str,
        user: &str,
        password: &str,
    ) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::SetupForeignCoin {
            coin_name: coin_name.to_owned(),
            user: user.to_owned(),
            password: password.to_owned(),
        })
        .await
    }

    /// Maps public port 1222 to the player's reverse-shell service.
    pub async fn forward_rshell(&self, local_ip: &str) -> Result<(), TestClientError> {
        match self
            .request(RequestKind::ForwardService {
                local_ip: local_ip.to_owned(),
                public_port: 1222,
                internal_port: 1222,
                service: "rshell".to_owned(),
            })
            .await?
        {
            Reply::Applied => Ok(()),
            _ => Err(TestClientError::UnexpectedResponse("Applied")),
        }
    }

    /// Relocks override settings for the smart appliance at `local_ip` in the player's network.
    pub async fn lock_appliance(&self, local_ip: &str) -> Result<(), TestClientError> {
        match self
            .request(RequestKind::LockAppliance {
                local_ip: local_ip.to_owned(),
            })
            .await?
        {
            Reply::Applied => Ok(()),
            _ => Err(TestClientError::UnexpectedResponse("Applied")),
        }
    }

    /// Sets a computer's flood counter so the next scripted flood triggers a reboot.
    pub async fn prime_flood(&self, local_ip: &str) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::PrimeFlood {
            local_ip: local_ip.to_owned(),
        })
        .await
    }

    /// Freezes the game clock at the current minute for date comparison.
    pub async fn freeze_clock(&self) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::FreezeClock).await
    }

    /// Marks the network at `public_ip` unpaid and disabled, or restores it.
    pub async fn disable_network(
        &self,
        public_ip: &str,
        disabled: bool,
    ) -> Result<(), TestClientError> {
        match self
            .request(RequestKind::DisableNetwork {
                public_ip: public_ip.to_owned(),
                disabled,
            })
            .await?
        {
            Reply::Applied => Ok(()),
            _ => Err(TestClientError::UnexpectedResponse("Applied")),
        }
    }

    /// Grants super-administrator status to the administrator of the network at `public_ip`.
    pub async fn promote_admin(&self, public_ip: &str) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::PromoteAdmin {
            public_ip: public_ip.to_owned(),
        })
        .await
    }

    /// Dismisses NPC `user` from the network at `public_ip`.
    pub async fn fire_npc(&self, public_ip: &str, user: &str) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::FireNpc {
            public_ip: public_ip.to_owned(),
            user: user.to_owned(),
        })
        .await
    }

    /// Has NPC `user`'s workstation in the network at `public_ip` refresh and save its account of the NPC, as the
    /// game does the next time a player connects to it.
    pub async fn refresh_npc_account(
        &self,
        public_ip: &str,
        user: &str,
    ) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::RefreshNpcAccount {
            public_ip: public_ip.to_owned(),
            user: user.to_owned(),
        })
        .await
    }

    /// Gives NPCs of the network at `public_ip` a company stance and a zero-day contact
    /// (`NetworkLan.ConfigZeroDayInfo`).
    pub async fn config_zero_day_info(&self, public_ip: &str) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::ConfigZeroDayInfo {
            public_ip: public_ip.to_owned(),
        })
        .await
    }

    /// Uses the game's alarm methods to change the alarm on NPC `user`'s smartphone.
    pub async fn set_phone_alarm(
        &self,
        public_ip: &str,
        user: &str,
        hour: i64,
        minute: i64,
        delete_alarm: bool,
    ) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::SetPhoneAlarm {
            public_ip: public_ip.to_owned(),
            user: user.to_owned(),
            hour,
            minute,
            delete_alarm,
        })
        .await
    }

    /// Freezes the police network during a driver's commute and activates its traffic override.
    pub async fn setup_traffic(&self) -> Result<TrafficReadyReply, TestClientError> {
        match self.request(RequestKind::SetupTraffic).await? {
            Reply::TrafficReady(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("TrafficReady")),
        }
    }

    /// Invokes the game's car-accident RPC for the most recently located driver.
    pub async fn traffic_accident(
        &self,
        user: &str,
        public_ip: &str,
        police_ip: &str,
    ) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::TrafficAccident {
            user: user.to_owned(),
            public_ip: public_ip.to_owned(),
            police_ip: police_ip.to_owned(),
        })
        .await
    }

    /// Expires the traffic cooldown while retaining the consumed override and dead driver.
    pub async fn reset_traffic_cooldown(&self) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::ResetTrafficCooldown).await
    }

    /// Hires an administrator for the rented-server network at `public_ip`.
    pub async fn hire_rental_admin(&self, public_ip: &str) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::HireRentalAdmin {
            public_ip: public_ip.to_owned(),
        })
        .await
    }

    /// Makes the network at `public_ip` one the player rents, charged `amount` a month by the player's rental
    /// payment (`BankHelperServer.RentServer`, `BankSubs.AddPayment`).
    pub async fn rent_network(&self, public_ip: &str, amount: i64) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::RentNetwork {
            public_ip: public_ip.to_owned(),
            amount,
        })
        .await
    }

    /// Hires (`hire`) or removes the administrator of the rented network at `public_ip` through the bank's page,
    /// paying and registering the monthly charge or taking it off
    /// (`BankHelperServer.HireAdminRentalServerRpc`, `FireAdminRentalServerRpc`).
    pub async fn serve_rental_admin(
        &self,
        public_ip: &str,
        hire: bool,
    ) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::RentalAdminService {
            public_ip: public_ip.to_owned(),
            hire,
        })
        .await
    }

    /// Runs each of the player's monthly payments now (`BankSubs.Payment.RunPayment`).
    pub async fn run_payments(&self) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::RunPayments).await
    }

    /// Runs the player's monthly payment `kind` (`BANK`, `RENTED` or `ADMIN`) alone, as the game does when its own
    /// date comes (`BankSubs.Payment.RunPayment`).
    pub async fn run_payment(&self, kind: &str) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::RunPayment {
            kind: kind.to_owned(),
        })
        .await
    }

    /// Sets the money of the player's bank account (`BankAccount.SetMoney`).
    pub async fn set_player_money(&self, money: f64) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::SetPlayerMoney { money })
            .await
    }

    /// Logs in the administrator of the network at `public_ip` for one hour, matching social-engineering mail.
    pub async fn force_admin_login(&self, public_ip: &str) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::ForceAdminLogin {
            public_ip: public_ip.to_owned(),
        })
        .await
    }

    /// Advances the game clock by `seconds` and returns the resulting game date.
    pub async fn advance_clock(&self, seconds: u64) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::AdvanceClock { seconds })
            .await
    }

    /// Restarts the player's computer from a terminal connected to it, as `ComputerIntrinsics.reboot` does (with
    /// `safe_mode` for `reboot(1)`), and returns once the game's client has started it again.
    pub async fn reboot_player(&self, safe_mode: bool) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::RebootPlayer { safe_mode })
            .await
    }

    /// Returns the public address of a new `kind` network spawned through the game without a generated address.
    pub async fn spawn_network(&self, kind: &str) -> Result<NetworkCreatedReply, TestClientError> {
        match self
            .request(RequestKind::SpawnNetwork {
                kind: kind.to_owned(),
            })
            .await?
        {
            Reply::NetworkCreated(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("NetworkCreated")),
        }
    }

    /// Creates a CTF-kind network before the player joins it and returns its addresses.
    pub async fn spawn_ctf_network(&self) -> Result<NetworkCreatedReply, TestClientError> {
        match self.request(RequestKind::SpawnCtfNetwork).await? {
            Reply::NetworkCreated(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("NetworkCreated")),
        }
    }

    /// Sets every firewall rule on the router or switch at `local_ip` in the player's network to allow.
    pub async fn allow_firewall(&self, local_ip: &str) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::AllowFirewall {
            local_ip: local_ip.to_owned(),
        })
        .await
    }

    /// Draws a group of addresses for mission `index` of the kind with the game's `TypeMission` name `kind`, and
    /// returns the mission the game builds for it and the networks it generated for the group.
    pub async fn mission(&self, kind: &str, index: u32) -> Result<MissionReply, TestClientError> {
        match self
            .request(RequestKind::Mission {
                kind: kind.to_owned(),
                index,
            })
            .await?
        {
            Reply::Mission(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("Mission")),
        }
    }

    /// Returns the exploits of the game's exploit database for the library file `library` at `version`.
    pub async fn query_exploits(
        &self,
        library: &str,
        version: &str,
    ) -> Result<ExploitsReply, TestClientError> {
        match self
            .request(RequestKind::QueryExploits {
                library: library.to_owned(),
                version: version.to_owned(),
            })
            .await?
        {
            Reply::Exploits(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("Exploits")),
        }
    }

    /// Buys exploit `exploit` into `folder` of the player computer through the game's shop and download RPCs.
    pub async fn buy_exploit(
        &self,
        library: &str,
        version: &str,
        exploit: &str,
        folder: &str,
        compile: bool,
    ) -> Result<ExploitBoughtReply, TestClientError> {
        match self
            .request(RequestKind::BuyExploit {
                library: library.to_owned(),
                version: version.to_owned(),
                exploit: exploit.to_owned(),
                folder: folder.to_owned(),
                compile,
            })
            .await?
        {
            Reply::ExploitBought(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("ExploitBought")),
        }
    }

    /// Has the administrator of the network at `public_ip` report the newest library file `library` as its network's
    /// update (`Persona.UpdateReportedNetworks`), and returns the library the game reported.
    pub async fn report_library_update(
        &self,
        public_ip: &str,
        library: &str,
    ) -> Result<LibraryReportedReply, TestClientError> {
        match self
            .request(RequestKind::ReportLibraryUpdate {
                public_ip: public_ip.to_owned(),
                library: library.to_owned(),
            })
            .await?
        {
            Reply::LibraryReported(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("LibraryReported")),
        }
    }

    /// Releases the next version of the library file `library` in the game, as it does for a world's first versions
    /// (`VersionsControl.CheckUpdatePrograms`), and returns the game's newest library and metaxploit before and after.
    pub async fn release_library_version(
        &self,
        library: &str,
    ) -> Result<LibraryReleasedReply, TestClientError> {
        match self
            .request(RequestKind::ReleaseLibraryVersion {
                library: library.to_owned(),
            })
            .await?
        {
            Reply::LibraryReleased(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("LibraryReleased")),
        }
    }

    /// Adds a guild with the game's guild types: see [`GuildSetup`].
    pub async fn setup_guild(&self, guild: GuildSetup) -> Result<(), TestClientError> {
        self.apply_fixture(RequestKind::SetupGuild(guild)).await
    }

    /// Reads the game's guilds, the share holders of the companies at `company_ips` and the networks CrownShield
    /// protects.
    pub async fn query_guilds(
        &self,
        company_ips: &[String],
    ) -> Result<GuildsReply, TestClientError> {
        match self
            .request(RequestKind::QueryGuilds {
                company_ips: company_ips.to_vec(),
            })
            .await?
        {
            Reply::Guilds(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("Guilds")),
        }
    }

    /// Reads the player's active traces, the computers the passive trace has queued and the strikes.
    pub async fn query_traces(&self) -> Result<TracesReply, TestClientError> {
        match self.request(RequestKind::QueryTraces).await? {
            Reply::Traces(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("Traces")),
        }
    }

    /// Gives the player strike `strikes` as the end of an active trace on the company at `company_ip` does
    /// (`PlayerHelperServer.ActiveGameOver`), and returns what the game has of the player afterwards.
    pub async fn end_game(
        &self,
        strikes: u32,
        company_ip: &str,
    ) -> Result<GameEndedReply, TestClientError> {
        match self
            .request(RequestKind::EndGame {
                strikes,
                company_ip: company_ip.to_owned(),
            })
            .await?
        {
            Reply::GameEnded(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("GameEnded")),
        }
    }

    /// Reads the game's name lists as `WordGenerator` loads them.
    pub async fn query_word_lists(&self) -> Result<WordListsReply, TestClientError> {
        match self.request(RequestKind::QueryWordLists).await? {
            Reply::WordLists(result) => Ok(result),
            _ => Err(TestClientError::UnexpectedResponse("WordLists")),
        }
    }

    /// Sends `request`; the plugin responds by writing a file.
    async fn apply_fixture(&self, request: RequestKind) -> Result<(), TestClientError> {
        match self.request(request).await? {
            Reply::Applied => Ok(()),
            _ => Err(TestClientError::UnexpectedResponse("Applied")),
        }
    }
}

/// Current state of a script started by [`TestClient::execute_until_input`].
#[derive(Debug)]
pub enum ExecutionStep {
    /// The script awaits input for [`TestClient::type_input`].
    Input,
    /// The script exited.
    End(ExecutionResultReply),
}

impl ExecutionStep {
    fn of(reply: Reply) -> Result<Self, TestClientError> {
        match reply {
            Reply::InputRequested => Ok(Self::Input),
            Reply::ExecutionResult(result) => Ok(Self::End(result)),
            _ => Err(TestClientError::UnexpectedResponse(
                "InputRequested or ExecutionResult",
            )),
        }
    }
}
