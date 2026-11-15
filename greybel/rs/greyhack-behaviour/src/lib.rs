//! Grey Hack rules shared by program runtimes and the game server, including paths, addresses,
//! credentials, limits, and service names. Runtimes can apply these rules without world access.
//!
//! The rules live in domain folders (`base`, `computer`, `network`, `economy`, `npcs`, `guilds`,
//! `player`) and are re-exported from the crate root.

#[macro_use]
mod enum_def;
mod base;
mod computer;
mod economy;
mod guilds;
mod network;
mod npcs;
pub mod path;
mod player;

// Foundations the other rules build on: the seeded generator, the clock, the world seed, refusals, versions, and generated words.
pub use base::clock::{
    MILLIS_PER_SECOND, SECONDS_PER_DAY, SECONDS_PER_HOUR, SECONDS_PER_MINUTE, START_DATE,
    TIME_SCALE, add_month, add_months, advance_ticks, date_parts, day_of_year,
    format_auto_patch_date, format_date, format_file_date, is_weekend, parse_date,
    time_after_ticks, try_parse_date, world_seconds,
};
pub use base::refusal::{Credential, NameKind, Refusal, RuleResult};
pub use base::rng::{ALPHANUMERIC, HEX_DIGITS, LOWERCASE_ALPHANUMERIC, Rng, fnv1a_64};
pub use base::versions::{FIRST_VERSION, compare_versions, is_valid_version};
pub use base::words::{Gender, Word};
pub use base::world_seed::{WORLD_SECRET_LENGTH, WorldSeed};
// Computers: processes, accounts, permissions, files, startup, logs, libraries, packages, and firewalls.
pub use computer::apt::{
    APT_BINARY_FOLDER, APT_CACHE_NAME, APT_CACHE_PATH, APT_FOLDER, APT_LIBRARY_FOLDER,
    APT_PACKAGE_SIZE, APT_SOURCES_PATH, AptSource, DEFAULT_REPOSITORY, DEFAULT_REPOSITORY_PORT,
    OFFICIAL_OMISSIONS, OFFICIAL_SERVER, default_apt_install_folder, format_apt_sources,
    parse_apt_official, parse_apt_sources,
};
pub use computer::computers::{
    CLOCK_PROGRAM_NAME, CLOCK_PROGRAM_PATH, DeviceKind, FIRST_SPAWN_PID, FLOOD_REBOOT_MINUTES,
    MAX_LAUNCH_DEPTH, MAX_PROCESS_NAME, MAX_TERMINALS, MAX_USERS, PROCESS_RAM_MB, PROTECTED_PIDS,
    REBOOT_MINUTES, RESERVED_PROCESS_NAMES, SAFE_MODE_TERMINAL, SAFE_MODE_TERMINAL_RAM_MB,
    SESSION_PROCESS, SYSTEM_PROCESSES, check_process_name, is_system_process, reboot_seconds,
    session_pid,
};
pub use computer::credentials::{
    AccountRefusal, CredentialError, GUEST_USER, MAX_CREDENTIAL, MAX_CTF_PASSWORD,
    MAX_DEBUG_LOGIN_ATTEMPTS, MAX_WALLET_CREDENTIAL, ROOT_USER, check_account_capacity,
    check_account_name, check_ctf_password, check_new_group, check_new_subwallet, check_new_user,
    check_new_wallet, check_password, check_wallet_password, hash_password, passwd_content,
    validate_credential, validate_wallet_credential, verify_password,
};
pub use computer::debug_library::{
    auto_patch_time, error_lines_match, fixed_payload_user, is_patch_applied, is_rumor_time,
};
pub use computer::filesystem::file_data::FileData;
pub use computer::filesystem::kinds::{Command, FileKind};
pub use computer::filesystem::limits::{
    BYTES_PER_MB, MAX_CONTENT, MAX_FILES, MAX_FILES_PER_FOLDER, MAX_IMPORTED_CODE, MAX_NAME,
    MAX_SYMLINKS, METADATA_SIZE, check_content, check_file_name, check_folder_name, check_new_name,
    check_target_name,
};
pub use computer::filesystem::paths::{
    BASH_PATH, DOWNLOADS_PATH, PASSWD_FOLDER, PASSWD_NAME, PASSWD_PATH, WEBSITE_PATH,
    bank_config_path, home_folders, mail_config_path,
};
pub use computer::filesystem::timestamps::{template_timestamp, timestamp_hash};
pub use computer::firewall::{FirewallRule, firewall_allows};
pub use computer::hardware::{GpuFault, HardwareKind, check_gpu, interface_names};
pub use computer::imports::{
    Import, ImportedCodeTooLarge, UnclosedImport, import_paths, put_imports,
};
pub use computer::libraries::{
    KERNEL_ROUTER_PATH, LIBRARY_FOLDER, LibraryClass, LibraryKind, LibraryRole, Requirement,
    Reward, library_path,
};
pub use computer::library_versions::{
    LIBRARY_CHECK_MILLIS, MAX_GENERATED_ZONES, MAX_SAVED_LIBRARIES, MAX_WEAKNESS_USES,
    MAX_ZONE_PATCHES, count_use, is_worn_out, is_zone_exhausted, library_check_seconds,
    next_version, replacement_zone_count, saved_versions_to_prune,
};
pub use computer::permissions::{FileOwnership, Permission, PermissionSet, Principal};
pub use computer::smart_appliance::SmartApplianceSettings;
pub use computer::ssh::{
    SSH_SCRIPT_FOLDER, SSH_SCRIPT_PARAM, SSHD_CONFIG_PATH, before_end, end_result,
};
pub use computer::startup::{ROUTER_STARTUP_FILES, STARTUP_FILES, StartupContent, StartupRole};
pub use computer::system_log::{
    LogAction, LogEntry, MAX_LOG_ENTRIES, SYSTEM_LOG_FOLDER, SYSTEM_LOG_FOLDER_PERMISSIONS,
    SYSTEM_LOG_NAME, SYSTEM_LOG_PATH, SYSTEM_LOG_PERMISSIONS, SystemLog, player_net_id,
    previous_address, token_trace,
};
// Money: coins, markets, rentals, and the hack shop.
pub use economy::blockchain::{
    COUPON_VALUE, GAME_ACCOUNT, INITIAL_COUPONS, MAX_BANK_TRANSACTIONS, PRICE_HISTORY_LIMIT, Side,
    TRANSFER_FEE_PERCENT, admin_payment_ends, coupon_cost, cycle_seconds, hash_power,
    order_book_price, payment_ends, pending_trade_price, pick_by_power, pick_machine, round_reward,
    transfer_fee,
};
pub use economy::coin_limits::{CoinLimit, MAX_COINS, MAX_CYCLE_HOURS};
pub use economy::exploit_shop::{
    ExploitPart, ExploitSpec, exploit_description, exploit_file_name, exploit_part_choices,
    exploit_price, exploit_source, unique_exploit_name,
};
pub use economy::market::{
    MAX_BALANCE, MAX_ORDER_COINS, MAX_ORDER_PRICE, MIN_TRANSFER_AMOUNT, NetworkEffect,
    RENTAL_ADMIN_PRICE, RENTAL_COMPUTER_PRICE, RENTAL_HUB_PRICE, RENTAL_ROUTER_PRICE,
    RENTAL_SWITCH_PRICE, RentalDeviceTable, SubscriptionKind, cap_balance, check_order,
    clamp_balance, net_speed_price, rental_price, rental_price_without_admin,
    valid_transfer_amount,
};
pub use economy::shop_items::{HACK_SHOP_LIBRARIES, detail_key, shop_programs};
pub use enum_def::UnknownCode;
// Guilds, company rankings, and traces.
pub use guilds::ownership::{
    ADMIN_ABSENCE_DAILY_RECOVERY_PERCENT, ADMIN_ABSENCE_PENALTY_PERCENT, CROWN_QUALIFICATION_DAYS,
    CROWN_SHIELD_PRICE, DDOS_LEADERBOARD_FACTOR, DDOS_LEADERBOARD_PENALTY_HOURS,
    DDOS_LEADERBOARD_PENALTY_SECONDS, MAJORITY_PERCENT, add_days, company_share_price,
    default_company_seed, effective_max_admin_restore_disabled_companies,
    effective_max_invested_companies, exp_after_member_exit, is_listed_on_stock_market,
    leaderboard_points, majority_owner, max_admin_restore_companies_for_level,
    max_admin_restore_companies_for_perks, max_invested_companies_for_level,
    max_invested_companies_for_perks, max_share_price, max_stocks, next_day, reputation_level,
};
pub use guilds::ranked_company::{RankedCompany, company_weight, guild_points, order_companies};
pub use guilds::ranked_guild::{RankedGuild, crown_leader, sort_leaderboard};
pub use guilds::traces::{
    ACTIVE_TRACE_SECONDS, MAX_PASSIVE_TRACES, MAX_STRIKES, PASSIVE_BASE_MS, PASSIVE_BOUNCE_MS,
    PASSIVE_NEXT_ADMIN_MS, PassiveStage, StrikeCount, active_trace_end, active_trace_seconds,
    add_strike, passive_trace_delay_ms, requeues_absent_admin, should_start_tracking,
};
// Networks: addresses, LANs, network kinds and generation, domains, Wi-Fi, services, and traffic.
pub use network::address::{
    LOCAL_RANGES, NO_ADDRESS, is_group_reserved_ip, is_lan_ip, is_public_ip, is_reserved_ip,
    is_valid_ip, is_valid_lan_ip, is_valid_unreserved_ip, lan_hosts, main_router_address,
    same_subnet,
};
pub use network::connections::{FORBIDDEN_PARAM_TOKENS, forbidden_param_token};
pub use network::domains::{
    LetterTableSet, ROUTER_NODE_LABEL, TOP_LEVEL_DOMAINS, WEB_PREFIX, clean_domain_name,
    network_domain, unique_name_to_ip, web_domain,
};
pub use network::generation::{DeviceDraw, NPCS_PER_NETWORK, admin_candidates};
pub use network::lan::{Lan, LanFailure, LanNode, LanNodeId, LanPort, is_device};
pub use network::networks::{
    DEFAULT_NET_SPEED, KERNEL_ROUTER, MAX_NETWORK_COMPUTERS, NET_SPEEDS, NetworkKind,
    flood_reboot_threshold, kind_for_ip, map_distance, position_for_ip, primary_services,
};
pub use network::services::{
    COMPUTER_SERVICES, MAX_REVERSE_SHELLS_PER_PORT, ROUTER_PORT, ServiceId,
};
pub use network::traffic::{
    ACCIDENT_COOLDOWN_SECONDS, license_plate, license_plate_address, normalize_license_plate,
};
pub use network::wifi::{WIFI_RANGE, collect_acks, required_acks, signal_strength, wifi_power};
// NPCs: schedules, roles, lateness, administrators, and mail.
pub use npcs::admin_absence::{AdminAbsence, admin_absent};
pub use npcs::admin_arrival::{AdminArrival, admin_arrives};
pub use npcs::alarm_config::AlarmConfig;
pub use npcs::company_align::CompanyAlign;
pub use npcs::job_role::JobRole;
pub use npcs::late_wake::{LateWake, plan_late_wake};
pub use npcs::lateness::{
    HIRED_RECENTLY_DAYS, LATE_STRIKES_TO_FIRE, Lateness, fires_for_lateness, is_hired_recently,
    lateness, pending_wake_up,
};
pub use npcs::mail::{
    DEFAULT_MAIL_DOMAIN, MAIL_PREVIEW, MAX_MAIL_ADDRESS, MAX_MAIL_MESSAGE, MAX_MAIL_SUBJECT,
    META_MAIL_COOLDOWN, check_mail, npc_mail_address,
};
pub use npcs::network_admin::NetworkAdmin;
pub use npcs::npc_roles::{assign_role, draw_plate};
pub use npcs::npc_schedule::{
    COMMUTE_SECONDS, DEATH_DELAY_SECONDS, FORCED_LOGIN_SECONDS, NpcSchedule,
};
pub use npcs::work_performance::{WorkPerformance, average_work_performance};
// What the player does: CTF events and missions.
pub use player::ctf::{
    MARK_RANGE, creator_reward, creator_score, player_reward, quality_score, round_even,
};
pub use player::mission_groups::{
    GROUP_COUNT, MIN_ZERO_DAY_GROUP, MissionKind, ip_group, mission_for_group,
    random_group_for_mission,
};
