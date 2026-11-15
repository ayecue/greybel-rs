//! Values the evidence functions of procedural missions draw (`ProcMissionBuilder.Add*TagMap`, `Generate*TagMap`) and
//! the random values they use (`IP.GetRandomPublicIP`, `IP.GetRandomLocalIP`, `WordGenerator`).

use greyhack_behaviour::{
    ALPHANUMERIC, Gender, LOCAL_RANGES, SECONDS_PER_DAY, START_DATE, Word, format_file_date,
    is_group_reserved_ip, parse_date,
};
use mono_random::Random;

use super::model::TagMap;

/// `System.Random.Next(min, max)`.
pub fn between(random: &mut Random, min: i32, max: i32) -> i32 {
    if max <= min {
        min
    } else {
        min + i32::try_from(
            random.next_below(usize::try_from(max - min).expect("the tag range is positive")),
        )
        .expect("value fits the target integer type")
    }
}

/// `WordGenerator.GetNextWord`.
pub fn next_word(word: Word, random: &mut Random) -> String {
    let text = word.generate_with_index(|length| random.next_below(length));

    if word == Word::Password && random.next_below(2) == 1 {
        text.to_lowercase()
    } else {
        text
    }
}

/// `WordGenerator.GetNombre` for a gender drawn as the missions do.
pub fn next_first_name(random: &mut Random) -> String {
    let gender = if random.next_double() > 0.5 {
        Gender::Female
    } else {
        Gender::Male
    };

    gender.first_name_with_index(|length| random.next_below(length))
}

/// `IP.GetRandomPublicIP`.
pub fn random_public_ip(random: &mut Random) -> String {
    loop {
        let first = between(random, 1, 256);
        let second = between(random, 0, 256);
        let third = between(random, 0, 256);
        let fourth = between(random, 1, 255);
        let address = format!("{first}.{second}.{third}.{fourth}");

        if !is_group_reserved_ip(&address) {
            return address;
        }
    }
}

/// `IP.GetRandomLocalIP`.
pub fn random_local_ip(random: &mut Random) -> String {
    let range = LOCAL_RANGES[random.next_below(LOCAL_RANGES.len())];
    let third = between(random, 0, 26);
    let fourth = between(random, 2, 21);

    format!("{range}.{third}.{fourth}")
}

/// `ProcMissionBuilder.GenerateRandomAlphaNumeric`.
pub fn random_alphanumeric(random: &mut Random, length: usize) -> String {
    (0..length)
        .map(|_| char::from(ALPHANUMERIC[random.next_below(ALPHANUMERIC.len())]))
        .collect()
}

fn put(tags: &mut TagMap, key: &str, value: &impl ToString) {
    tags.insert(key.to_owned(), value.to_string());
}

/// `ProcMissionBuilder.AddTransferBalanceTags`.
pub fn transfer_balance(tags: &mut TagMap, random: &mut Random) {
    for letter in ['A', 'B', 'C'] {
        let start = between(random, 10, 31);
        let inflow = between(random, 10, 31);
        let end = between(random, 1, 11);
        let extra = if letter == 'B' {
            between(random, 1, 10)
        } else {
            0
        };

        put(tags, &format!("$TRANSFER_OPENING_{letter}"), &start);
        put(tags, &format!("$TRANSFER_INCOMING_{letter}"), &inflow);
        put(
            tags,
            &format!("$TRANSFER_RECORDED_{letter}"),
            &(start + inflow - end - extra),
        );
        put(tags, &format!("$TRANSFER_CLOSING_{letter}"), &end);
    }
}

/// `ProcMissionBuilder.AddSplitPaymentTags`.
pub fn split_payment(tags: &mut TagMap, random: &mut Random) {
    let total = between(random, 40, 151) * 10;
    let fee = between(random, 1, 7) * 5;

    put(tags, "$SPLIT_PAYMENT_TOTAL", &total);
    put(tags, "$SPLIT_PAYMENT_FEE", &fee);

    for (index, amount) in [total, total - 2 * fee, total - fee]
        .into_iter()
        .enumerate()
    {
        let first = between(random, 1, amount);
        let letter =
            char::from(b'A' + u8::try_from(index).expect("value fits the target integer type"));

        put(tags, &format!("$SPLIT_PAYMENT_FIRST_{letter}"), &first);
        put(
            tags,
            &format!("$SPLIT_PAYMENT_SECOND_{letter}"),
            &(amount - first),
        );
    }
}

/// `ProcMissionBuilder.AddBlockCoverageTags`.
pub fn block_coverage(tags: &mut TagMap, random: &mut Random) {
    let first = between(random, 3, 9);
    let second = between(random, 3, 9);
    let third = between(random, 3, 9);
    let total = first + second + third;
    let overlap_first = between(random, 1, first.min(second));
    let overlap_second = between(random, 1, second.min(third));
    let range = |start: i32, count: i32| format!("{start}-{}", start + count - 1);
    let one = range(0, first);
    let two = range(first, second);
    let three = range(first + second, third);

    put(tags, "$BLOCK_COVERAGE_TOTAL", &total);
    put(tags, "$BLOCK_COVERAGE_LAST", &(total - 1));
    put(
        tags,
        "$BLOCK_COVERAGE_RANGES_A",
        &[
            one.clone(),
            range(first - overlap_first, second),
            three.clone(),
        ]
        .join(", "),
    );
    put(
        tags,
        "$BLOCK_COVERAGE_RANGES_B",
        &[three.clone(), one.clone(), two.clone()].join(", "),
    );
    put(
        tags,
        "$BLOCK_COVERAGE_RANGES_C",
        &[one, two, range(first + second - overlap_second, third)].join(", "),
    );
}

/// `ProcMissionBuilder.AddDigitChecksumTags`.
pub fn digit_checksum(tags: &mut TagMap, random: &mut Random) {
    let mut digits: Vec<u8> = format!("{:05}", between(random, 0, 100_000)).into_bytes();
    let sum: u32 = digits.iter().map(|digit| u32::from(digit - b'0')).sum();

    put(tags, "$DIGIT_CODE", &String::from_utf8_lossy(&digits));
    put(tags, "$DIGIT_CHECKSUM", &(sum % 10));

    let position = random.next_below(digits.len());
    let digit = u32::from(digits[position] - b'0');

    digits[position] = b'?';
    put(tags, "$DIGIT_PARTIAL", &String::from_utf8_lossy(&digits));

    let first = u32::try_from(between(random, 1, 10)).expect("the positive tag value fits");
    let mut second = u32::try_from(between(random, 1, 9)).expect("the positive tag value fits");

    if second >= first {
        second += 1;
    }

    digits[position] = b'0' + ((digit + first) % 10) as u8;
    put(tags, "$DIGIT_DECOY1", &String::from_utf8_lossy(&digits));
    digits[position] = b'0' + ((digit + second) % 10) as u8;
    put(tags, "$DIGIT_DECOY2", &String::from_utf8_lossy(&digits));
}

/// `ProcMissionBuilder.AddFileTimelineTags`: dates in the game's first month (`Clock.GetInitialDateTime`).
pub fn file_timeline(tags: &mut TagMap, random: &mut Random) {
    let midnight = parse_date(&format!(
        "{} - 00:00",
        START_DATE
            .split_once(" - ")
            .map_or(START_DATE, |(date, _)| date)
    ));
    let first = midnight
        - u64::try_from(between(random, 1, 31)).expect("the positive tag value fits")
            * SECONDS_PER_DAY
        + u64::try_from(between(random, 360, 960)).expect("the positive tag value fits") * 60;
    let second =
        first + u64::try_from(between(random, 20, 61)).expect("the positive tag value fits") * 60;
    let third =
        second + u64::try_from(between(random, 5, 26)).expect("the positive tag value fits") * 60;
    let fourth =
        third + u64::try_from(between(random, 5, 31)).expect("the positive tag value fits") * 60;
    let fifth =
        fourth + u64::try_from(between(random, 10, 61)).expect("the positive tag value fits") * 60;

    put(tags, "$FILE_TIMELINE_START_A", &format_file_date(first));
    put(tags, "$FILE_TIMELINE_START_B", &format_file_date(second));
    put(tags, "$FILE_TIMELINE_TIMESTAMP", &format_file_date(third));
    put(tags, "$FILE_TIMELINE_PASSWORD", &numeric_date(third, true));
    put(
        tags,
        "$FILE_TIMELINE_COVER_DATE",
        &numeric_date(third - SECONDS_PER_DAY, false),
    );
    put(tags, "$FILE_TIMELINE_START_C", &format_file_date(fourth));
    put(tags, "$FILE_TIMELINE_COPY", &format_file_date(fifth));
}

/// `DateTime.ToString("yyyyMMddHHmm")`, or `"yyyyMMdd"` without the time.
fn numeric_date(seconds: u64, with_time: bool) -> String {
    let (year, month, day) = greyhack_behaviour::date_parts(seconds);
    let within_day = seconds % SECONDS_PER_DAY;

    if with_time {
        format!(
            "{year:04}{month:02}{day:02}{:02}{:02}",
            within_day / 3_600,
            (within_day % 3_600) / 60
        )
    } else {
        format!("{year:04}{month:02}{day:02}")
    }
}

/// `ProcMissionBuilder.AddRevisionBoundaryTags`.
pub fn revision_boundary(tags: &mut TagMap, random: &mut Random) {
    let first = between(random, 100, 969);
    let last = first + 31;
    let change = between(random, first + 1, last);

    put(tags, "$REVISION_FIRST", &first);
    put(tags, "$REVISION_LAST", &last);
    put(tags, "$REVISION_CHANGE", &change);
}

/// `ProcMissionBuilder.GenerateClockOffsetTags`.
pub fn clock_offset(random: &mut Random) -> TagMap {
    let offset = between(random, 3, 13);
    let reference = between(random, 480, 840);
    let event = between(random, reference + 60, 1380);
    let time = |minutes: i32| format!("{:02}:{:02}", minutes / 60, minutes % 60);
    let mut tags = TagMap::new();

    put(&mut tags, "$CLOCK_REFERENCE_LOCAL", &time(reference));
    put(
        &mut tags,
        "$CLOCK_REFERENCE_SERVER",
        &time(reference + offset),
    );
    put(&mut tags, "$CLOCK_EVENT_LOCAL", &time(event));
    put(&mut tags, "$CLOCK_EVENT_SERVER", &time(event + offset));
    put(
        &mut tags,
        "$CLOCK_EVENT_MINUS_OFFSET",
        &time(event - offset),
    );
    put(
        &mut tags,
        "$CLOCK_EVENT_DOUBLE_OFFSET",
        &time(event + 2 * offset),
    );

    tags
}

/// `ProcMissionBuilder.GenerateOctetPermutationTags`.
pub fn octet_permutation(random: &mut Random) -> TagMap {
    let mut order = [0_usize, 1, 2, 3];

    loop {
        for at in (1..order.len()).rev() {
            let other = random.next_below(at + 1);

            order.swap(at, other);
        }

        if order != [0, 1, 2, 3] {
            break;
        }
    }

    let example = loop {
        let address = random_local_ip(random);
        let octets: Vec<String> = address.split('.').map(str::to_owned).collect();
        let mut distinct = octets.clone();

        distinct.sort();
        distinct.dedup();

        if distinct.len() == 4 {
            break octets;
        }
    };
    let mut tags = TagMap::new();

    put(&mut tags, "$OCTET_EXAMPLE_IP", &example.join("."));

    for (index, octet) in order.iter().enumerate() {
        let letter =
            char::from(b'A' + u8::try_from(index).expect("value fits the target integer type"));

        put(&mut tags, &format!("$OCTET_INDEX_{letter}"), octet);
        put(
            &mut tags,
            &format!("$OCTET_EXAMPLE_{letter}"),
            &example[*octet],
        );
    }

    tags
}
