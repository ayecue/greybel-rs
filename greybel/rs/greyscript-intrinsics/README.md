# GreyScript emulation

`greyscript-intrinsics` implements Grey Hack's GreyScript intrinsics against a
shared, persistent world. It is a de facto emulation for running and testing
scripts, not a claim of exact parity with every game code path.

The main pieces are:

- This crate: program-side natives, object behavior, builds and launches, input
  and terminal handling, and the interpreter host.
- [`greyhack-world`](../greyhack-world): seeded world generation, subsystem
  logic, deterministic routines, and Turso/SQLite persistence, independent of
  the interpreter.
- [`greyhack-world-operations`](../greyhack-world-operations): the duplex
  protocol server that serializes operations from connected programs against
  one world.
- [`greyhack-world-assets`](../../../assets/rs/greyhack-world-assets): embedded
  base filesystem data, command and service scripts, service definitions,
  names, address data, websites, and English game text.
- [`greyhack-behaviour`](../greyhack-behaviour): shared game constants and
  source-derived rules.

## Execution model and parity boundary

`GAME_API` registers the core API, player globals, and game object methods. The
object methods are also available as globals under their C# names, with aliases
such as `lan_ip`, `public_ip_pc`, `scan_debuglib`, and `reset_password_coin`.
World-reaching calls go through `ServerConnection` to `WorldServer`; they do not
read a second, interpreter-local copy of the world.

`WorldServer` executes operations, ticks, saves, and connection cleanup as
serialized cycles. A cycle settles reboot interruptions, ended processes and
jobs, object pinning, and the database commit before later work observes it.
Programs therefore share changes without observing partially applied
operations. Per-launch-tree operation budgets and throttle notifications keep
one busy program from monopolizing the server. Object handles, process-end
notifications, terminal output, and deferred job results travel over the same
protocol.

The emulation covers script-visible state and operations. It does not provide
the game's GUI applications, hardware-installation UI, real listening network
services, or general-purpose IP traffic. Service protocols exist where an
intrinsic models them; they are not socket servers. Some output and timing is
intentionally normalized in differential tests, so passing those tests is not
proof of parity outside their coverage.

## Seeded worlds and assets

`World::generate` takes a numeric seed and a requested number of additional
networks; `World::from_seed` hashes text with FNV-1a. `World::new` creates only
the player's network. `WorldOptions::default` uses seed zero and five additional
networks.

The world RNG is SplitMix64. Its state is saved with the world, subsystem forks
limit unrelated random-stream disturbances, and stable derived seeds cover
content that must survive independent generation order. Algorithms that depend
on .NET randomness use the Mono-compatible implementation. The persisted world
secret keys DNS and licence-plate derivation. Identical options produce
identical generated worlds; different operation or tick order can of course
produce different subsequent states.

The default `PlayerOptions` is a sandbox fixture, not an exact new-game player:
it supplies `test`/`test`, mail and bank accounts, extra libraries, 2048 MB RAM,
and a 10,000 MB disk. Options can instead describe a new player without those
conveniences or mirror an exact network layout. Generated networks contain
typed computers, routers, switches, hubs, cameras, smartphones, smart
appliances, services, libraries, hardware, NPCs, and scheduled work.

The embedded asset bundle supplies the game's base filesystem and command
sources rather than placeholders for every file. Service installation uses the
extracted service table, SSH encode/decode sources, default and type-specific
websites, repository packages, and record-viewer launchers. `include_lib`
requires a file whose stored kind and metadata identify a library; a text file
named `*.so` is not enough. `admin.so` is a separate sandbox exception described
below.

## World clock, ticks, and scheduled work

The world clock is independent of the host calendar and advances only when the
host drives it:

- `World::advance_clock(seconds)` moves time without running work.
- `World::tick(seconds)` advances time and runs all routines due by then.
- `World::run_due(budget)` runs a bounded number without advancing time.
- `World::advance_ticks(ticks, rate)` converts ticks at the game's time scale of
  14 world seconds per real second, carrying fractional seconds between ticks.
- `WorldServer::tick()` runs ticks due on its supplied scheduler clock. Attaching
  a runtime also starts its cooperative ticker; hosts with a manual clock can
  drive the same method themselves.

`admin.tick(seconds)` is the script-facing test helper. It advances by whole
seconds, runs due routines, and returns the new elapsed-second value. It does
not make the host clock part of world generation.

Persistent `Routine` values are `ServiceRestart`, `NpcSwitch`, and `CoinRound`.
They retain due times across saves. Due routines run by type priority, due time,
and ID, with aging to prevent starvation; the server applies a per-tick budget.
`TickReport` contains executed routines, NPC events, and routine failures. The
server also performs daily upkeep for coin prices and due bank subscriptions.
When its host is late, it processes a bounded catch-up window and jumps over
older ticks rather than trying to run an unbounded backlog.

VM timing is related but separate. `wait`, `poll_input`'s 250 ms suppression,
Wi-Fi reconnection, camera-link throttling, and MetaMail's two-second cooldown
use the interpreter/host clock. These throttles must not be implemented by
reading a native clock inside world generation or by spawning a thread per
script.

## Persistence and reopen behavior

Durable state lives in a Turso database: networks, files, users, hardware,
ports, processes selected to survive, libraries, banks and markets, mail, NPCs,
CTF and debug state, routines, RNG state, world clock, and traffic cooldown.
`World::commit` writes world metadata and commits the active transaction;
`save_into` copies the complete database. `WorldServer` commits at cycle
boundaries and `admin.snapshot` requests an explicit save when a backing
database exists.

Native `Database::open(path)` supports file-backed or in-memory databases. Web
builds always use in-memory storage, including when passed a path. The world
still has explicit commit points; browser storage is not durable across a page
reload unless the host exports and later restores the rows.

Opening a database creates missing tables with `CREATE TABLE IF NOT EXISTS`.
There is no general column-level migration or schema-version mechanism in the
current implementation. In particular, adding a column to an existing table is
not automatically handled by opening an older database; such a change requires
an explicit migration outside this schema initializer. Do not infer blanket
backward compatibility from the save/load tests, which use the current schema.

Reopening deliberately removes state tied to live programs:

- mining, sniffer, and flood registrations, plus mining-round routines;
- ordinary script processes and script login sessions.

Protected system processes, service daemons, reverse-shell clients and their
records, NPC login sessions, service-restart routines, and NPC-switch routines
survive. Pending mining jobs therefore do **not** resume after reopen, while
`show_nodes` correctly counts jobs during a live session. Sniffer packets are
produced as deferred results rather than a persisted packet log. The most
recently located traffic vehicle is session-local and is not restored.

## Networking, DNS, Wi-Fi, and repositories

Networks are stored as LAN trees with parent devices, gateways, local and public
addresses, port tables, forwards, firewall rules, and map positions.

- `get_router` resolves routers and switches by public or LAN address;
  `get_switch` accepts only switches. Contacting a valid, unreserved unknown
  public address generates its network once, and later calls reuse it.
- `nslookup` checks registered domains, with or without `www.`, then applies the
  world-secret-derived consonant/vowel encoding and .NET shuffle. `whois`
  exposes the network's web domain and administrative contact.
- Reachability checks public/LAN routing, port forwards, closed ports, startup
  files, reboot state, disabled networks, active cards and gateways, and every
  device on the required LAN path. A matching firewall allow rule overrides a
  deny rule.
- SSH and FTP service logins include account checks and the remote encode/decode
  script flow where configured. `scp` transfers through the same world
  filesystem and permission model.
- Wi-Fi discovery uses strict 0.5-unit range, nearest-first ordering, and
  distance-derived signal strength. Connecting validates the card and password,
  updates LAN membership, and has a one-second per-VM reconnect cooldown.
  Ethernet validates interface kind, address availability, and gateway. Moving
  between routers removes stale forwards to the computer.
- `airmon`, `aireplay`, and `aircrack` model monitor support, capture creation,
  ACK-dependent cracking, and password lookup without host radio access.
  `aireplay` output that depends on random ACK counts is compared in normalized
  form by differential tests.
- `aptclient` supports official and custom repository sources, cache updates,
  searches, installation, upgrade checks, and remote repository validation.

No native sockets are opened for these operations. They are deterministic
transitions and lookups in the emulated world.

## Files, builds, launches, processes, and services

The filesystem models users and groups, ownership, permissions, symlinks, file
kinds, metadata and content sizes, timestamps, and disk capacity. File content
is limited to 160,000 UTF-16 code units. Other source-derived limits include
node and per-folder entry counts, path/name lengths, and symlink depth.

`shell.build` reads source through the shell's computer and checks source and
destination permissions. It expands only the entry file's `import_code` calls,
requires absolute paths, enforces read and binary import permission, rejects
duplicate imports, and limits expanded input to 4,000,000 UTF-16 code units (not
4 MiB). An imported file's own `import_code` remains in the built source, where
it is not a valid runtime global. Built programs retain source maps and an
`allow_import` flag. Compilation uses the game-compatible strict language mode,
including its compiler-error forms.

`launch` starts a server-side process and enters a child context in the same VM
and heap. Launch trees share terminal output and the custom object, compiled
code is cached, and parent values remain alive across child execution and GC.
`launch_path` and `program_path` follow the launch chain. A child runtime failure
resets the custom object but does not make `launch` itself return false.
Launches charge 25 MB against installed RAM and reject nesting beyond 32 levels.

`start_terminal` runs the computer's executable `/bin/bash`, falling back to the
embedded game bash. It starts from the user's home and gives the terminal a new
custom object. This is terminal emulation, not a GUI terminal implementation.

Processes, login sessions (`dsession`), protected system processes, and service
daemons are visible to `show_procs` and resource checks.
Closing a process terminates the corresponding running interpreter context.
Reboots end processes and sessions, invalidate affected objects, and block
availability for three world minutes, or 20 after the relevant flooding path;
the player's protected system processes follow the game's special case.

Service installation writes the expected configuration and executable layout
and opens the port without adding a duplicate daemon. Explicit service start
and stop manage the daemon, and scheduled restarts run through world routines.
Repository, SSH, FTP, SMTP, HTTP, rshell, camera, smart-appliance, ADB, and SQL
record state are represented, but this is not a full implementation of every
application or protocol behind those services.

## Wallets, banking, trading, and mining

The economy supports:

- wallet creation/login, generated PINs, per-coin balances, and a strict
  one-day wallet password-reset interval;
- stable coin IDs, creator password changes, reward/cycle settings, global coin
  and price history queries, and deletion;
- subwallet creation, metadata, password checks, transfers with both sides'
  last-transaction records, and deletion with supply accounting;
- bank transfers with the game's 15% fee, transaction histories, monthly
  subscription records, and daily price updates from matched trades;
- oldest-first order matching, partial fills, pending-order cancellation,
  insufficient-funds and stale-offer handling, and pending trade-price averages.

`subwallet.mining` registers a job and waits for a coin round rather than always
returning immediately. Duplicate jobs for the same subwallet and computer are
rejected, while jobs for different computers can coexist. A round selects a
wallet weighted by its machines' total mining power, then selects one of that
wallet's machines uniformly. CPU speed/cores, RAM size/model, and GPU hashrate
contribute to power. One job wins the supply-capped reward; losers wait for the
next cycle. `show_nodes` counts pending jobs belonging to the wallet.

GPU health is persisted and checked at startup: very low health is damaged and
low health can fail the deterministic corruption check. The no-GPU CPU-decipher
path and broader hardware wear/degradation simulation are not modeled. Mining
depends on host-driven world ticks for deferred rounds; it does not require a
native mining thread. As noted above, live mining registrations and rounds are
cleared when a database is reopened.

## Metaxploit, sniffers, reverse shells, and crypto

`scan` and `scan_address` filter weaknesses by their required Metaxploit
version. Hidden-zone visibility follows the scanned library's version relative
to the world's latest version, including update warnings.

`net_use` resolves port zero to a router kernel and other ports to the service
library behind LAN or forwarded routing. `overflow` implements local file and
router attacks, requirement errors, and shell, computer, random-folder,
password-change, firewall-disable, appliance-override, and traffic-override
rewards. Checks include required libraries and versions, registered users,
forwards, gateway connections, active users, paths, and local subnets as
applicable. Network and rental administrators can intercept rewards and force
guest access. Router-kernel checks currently enforce device counts, subnet,
forward, and gateway requirements; other requirement kinds attached to a
kernel weakness are not enforced and pass.

`sniffer` waits for the next captured login. Capture selection follows the
source/destination device path, including routers and hub siblings. Successful
and failed service passwords, encrypted SSH handshakes, and social-engineering
NPC logins produce the game's source/destination/protocol and credential text.
`sniffer(1)` also writes the captured encoder as `encode.src` beside the running
program when permissions allow. Capture registrations are volatile, and this
is not a general packet sniffer for arbitrary protocols.

`rshell_client` validates process names, reaches an installed and forwarded
rshell service through the normal service/firewall checks, and records a shell
owned by the client process, up to 16 per server port. `rshell_server` returns
shells for live client processes and prunes stale records. `rshell_remove`
removes all shells on the service port or only a supplied list. Records persist
with those processes, and reboot or process cleanup invalidates them.

The crypto library also implements file encryption/decryption and the shared
password registry, hash deciphering, Wi-Fi capture/cracking, and SMTP user
enumeration against modeled services.

## NPCs, smart appliances, CCTV, and traffic

Generated networks include NPCs with computer accounts, mail/bank records,
phones, jobs, work schedules, and sometimes licence plates. Persistent NPC
routines log them in and out at work-block boundaries and update active-user
checks. Network administrators restore compromised generated networks when
they enter work. Social-engineering logins feed sniffers and can force an
off-shift NPC online for an hour.

Jobs include hiring into vacancies, firing and recent-hire restrictions,
police roles, and rented-network administrators whose attacks are intercepted.
Phone alarms affect lateness; repeated strikes can fire an NPC and deactivate
the account. Death and firing prevent future work logins.

Smart appliances expose model, override, alarm, and broken-state behavior.
Unsafe settings with the alarm disabled break the appliance for 24 world hours,
reset its override, block affected CCTV access, and schedule death for NPCs due
to walk to it within five minutes. The override exploit reward is required
before changing protected settings.

Police traffic networks select workstation credentials from police NPCs or the
administrator. `libtrafficnet.so` models credential information, camera-link
throttling, world-secret licence-plate decoding, and vehicle lookup only while
a living driver is commuting. An accident triggered through the admin helper
uses the last located vehicle, consumes the police override, kills the driver,
and starts a 12-hour persisted cooldown. The used override continues to block
cameras after cooldown expiry. Vehicle movement is schedule-based; full spatial
NPC travel is not simulated.

## CTF and debug/zero-day libraries

CTF state is persistent but empty unless created by a fixture or game snapshot.
Game intrinsics authenticate an event, expose description/template/creator/mail
content, complete a participant once and deliver completion mail, and reset the
account and owned-event passwords. Rating records quality and difficulty,
prevents duplicate or self-rating, and computes participant and creator bank
rewards with .NET round-to-even behavior. Creation and rating drivers are
admin-only helpers, not additional game intrinsics.

Debug libraries derive deterministic challenges from their library seed and
world clock. Implemented operations include account login and lockout,
inspection, unit-test error-line checks and persistent failure lockout,
manual/automatic patch state, whitespace-and-case-insensitive patch comparison,
and zero-day payload execution. Payloads return partial computer/file/library
objects with only the generated methods and reset the automatic-patch counter.
Zero-day addresses, requirements, rewards, objects, and variable names use the
game's lists, weighting, guest override, and .NET random stream. Differential
coverage is fixture-based; it does not establish equality for every generated
challenge or every remote credential flow.

## `admin.so`: sandbox-only helpers

`admin.so` is not a Grey Hack library. It is enabled by default in
`GreyHackInterpreter`, and `with_admin(false)` removes it to approximate a
normal game. Its methods exist to construct, inspect, and drive worlds from
tests and sandboxes; they must not be cited as game-intrinsic parity.

World-level helpers are `seed`, `snapshot`, `tick`, `set_date`,
`traffic_accident`, `networks`, `network`, `create_network`, `sudo`,
`create_program`, `exploit_shop`, `buy_exploit`, `register_password`,
`create_mail_account`, `create_ctf_account`, `accept_mission`,
`create_debug_account`, `deposit`, `grant_coins`, `create_ctf_event`,
`rate_ctf_event`, `create_guild`, `release_library_version`, and `report_exploit`.

`admin.network(public_ip)` returns an `adminNetwork` with `computer`,
`add_computer`, `fire_npc`, `hire_npc`, `hire_admin`, `fire_admin`, `remove`,
`set_net_speed`, `set_phone_alarm`, `phone_alarm`, `report_library_update`,
`set_company_shares`, `hire_crown_shield`, and `set_admin_restore`. Network and
computer addresses are properties of these handles rather than repeated method
arguments.

`network.computer(lan_ip)` returns an `adminComputer` with `shell`, `login`,
`npc_login`, `add_npc`, `add_net_card`, `add_smart_appliance`,
`set_library_version`, `set_port_library_version`, `service_records`,
`update_service_record`, `access_cctv`, and `install_smartphone_adb`.

Several helpers stand in for operations that the real game exposes through its
UI or server RPCs rather than GreyScript, including hiring, phone alarms,
accidents, CTF creation/rating, record editing, CCTV access, and ADB
installation. The helpers and the game intrinsics they set up are separate API
layers.

## Native and WebAssembly constraints

World and protocol logic is cooperative and same-thread oriented (`Rc`,
`RefCell`, and a local async scheduler). It does not require native threads,
native sockets, or a native wall clock. Hosts supply scheduling, clock, input,
output, and transport. Native applications can run the same in-memory server or
attach a file-backed database.

`wasm32-unknown-unknown` uses browser-compatible randomness and scheduler
support plus Turso's in-memory I/O adapter. Its monotonic time comes from the
web scheduler, while Turso's wall clock stays zero so host calendar time cannot
leak into world state. Web tests cover seeded generation, in-memory save/load,
and running programs with a same-thread server. Production Rust crates do not
depend on the .NET game harness.

## Real-game differential testing

[`greyhack-test-support-client`](../greyhack-test-support-client) runs parity
suites against Grey Hack and against this emulation. The .NET runner starts an
isolated headless clone with the message hook and a test-only support plugin;
the installed game is not modified. Its `testPort` serves the independent
`protocols/greyhack-test-support` bridge for typed fixture, state, and execution
operations. Plugin failures arrive as `Reply.TestFailure` and become
`TestClientError::Bridge` in Rust.

The parity support mirrors the selected game network's LAN, files, libraries,
users, processes, ports, hardware, NPCs, administrators, traffic and debug/CTF
state, world secret, date, mail, and global economy into a Rust world. Tests
freeze the game clock for snapshot comparisons and explicitly advance it for
schedule-dependent cases. Other cases compare real-time waits, two concurrent
programs, interactive input, or stepped world changes.

Coverage includes language/core behavior, objects and detached methods, files
and computers, builds and launches, terminal I/O, networks and encrypted
services, apt, Metaxploit and sniffers, reverse shells, blockchain, crypto,
NPCs, CTF, debug libraries, smart appliances, and traffic. Comparisons normally
include output, failure state, and runtime error text. Deliberate exceptions
include random ACK counts, game-selected `normal_user` names, and a few
launch/build failure texts where only post-failure behavior is compared.

For prerequisites, runner commands, shared game sessions, and the support
protocol, see [Game testing](../../../docs/game-testing.md). Local host-side test
helpers live in [`greybel-test-support`](../greybel-test-support); the .NET game
runner and plugin live in
[`greyhack-test-support`](../../dotnet/greyhack-test-support).
