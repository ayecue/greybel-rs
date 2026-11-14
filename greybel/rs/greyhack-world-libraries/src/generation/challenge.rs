//! Deterministic generation of flawed MiniScript functions for library debugging
//! (`ChallengeGenerator`, `CodeGenerator`).

use std::collections::BTreeSet;

use greyhack_behaviour::ALPHANUMERIC;
use mono_random::Random;

use crate::DebugChallenge;

const OPERATORS: [&str; 4] = ["+", "-", "*", "/"];
const REPLACEMENT_OPERATORS: [&str; 4] = ["-", "+", "/", "*"];
const KEYWORDS: [&str; 8] = [
    "function",
    "end function",
    "for",
    "end for",
    "if",
    "end if",
    "return",
    "then",
];

/// Generated source and deliberately injected errors (`ChallengeGenerator.GenerateChallenge`).
pub(crate) fn challenge(seed: i32) -> DebugChallenge {
    CodeGenerator::new(seed).generate()
}

struct CodeGenerator {
    random: Random,
    generated_functions: Vec<(String, String)>,
    additional_descriptions: Vec<String>,
    error_lines: BTreeSet<usize>,
    main_function: String,
    has_additional_function: bool,
    line_removal_complete: bool,
}

impl CodeGenerator {
    pub(crate) fn new(seed: i32) -> Self {
        Self {
            random: Random::new(seed),
            generated_functions: Vec::new(),
            additional_descriptions: Vec::new(),
            error_lines: BTreeSet::new(),
            main_function: String::new(),
            has_additional_function: false,
            line_removal_complete: false,
        }
    }

    pub(crate) fn generate(mut self) -> DebugChallenge {
        self.main_function = self.function_name();

        let parameter = self.variable_name();
        let mut description = String::new();
        let lines = self.function_body(&parameter, &mut description, true);
        let mut correct_code = format!("{} = function({parameter})\n", self.main_function);

        for line in lines {
            correct_code.push_str("    ");
            correct_code.push_str(&line);
            correct_code.push('\n');
        }

        correct_code.push_str("end function\n");

        for (_, function) in &self.generated_functions {
            correct_code.push('\n');
            correct_code.push_str(function);
        }

        let description = format!(
            "The function {} {}.",
            self.main_function,
            description.trim_end_matches('.')
        );
        let additional_descriptions = self.additional_descriptions.join("\n");
        let code = self.introduce_errors(&correct_code);

        DebugChallenge {
            description,
            additional_descriptions,
            code,
            correct_code,
            error_lines: self
                .error_lines
                .into_iter()
                .map(|line| line as i64)
                .collect(),
        }
    }

    pub(crate) fn function_body(
        &mut self,
        parameter: &str,
        description: &mut String,
        allow_function: bool,
    ) -> Vec<String> {
        let mut lines = Vec::new();
        let count = 3 + self.random.next_below(4);
        let current = parameter.to_owned();

        for _ in 0..count {
            match 1 + self.random.next_below(5) {
                1 => {
                    let (operation, operation_description) =
                        self.arithmetic(&current, lines.last().map(String::as_str));

                    lines.push(format!("{current} = {operation}"));
                    description.push_str(&format!("calculates {operation_description}. "));
                }
                2 => {
                    let (condition, condition_description) = self.condition(&current);
                    let (operation, operation_description) = self.arithmetic(&current, None);

                    lines.push(format!("if {condition} then"));
                    lines.push(format!("    {current} = {operation}"));
                    lines.push("end if".to_owned());
                    description.push_str(&format!(
                        "If {condition_description}, then updates {current} by calculating {operation_description}. "
                    ));
                }
                3 => {
                    let end = 2 + self.random.next_below(3);

                    lines.push(format!("for i in range(0, {end})"));
                    lines.push(format!("    {current} = i * {current}"));
                    lines.push("end for".to_owned());
                    description.push_str(&format!(
                        "For each i from 0 to {end}, updates {current} by multiplying it by i. "
                    ));
                }
                4 if allow_function && !self.has_additional_function => {
                    let (call, call_description) = self.function_call(&current);

                    lines.push(format!("{current} = {call}"));
                    description.push_str(&call_description);
                    description.push_str(". ");
                    self.has_additional_function = true;
                }
                4 => {
                    let (operation, operation_description) =
                        self.arithmetic(&current, lines.last().map(String::as_str));

                    lines.push(format!("{current} = {operation}"));
                    description.push_str(&format!("calculates {operation_description}. "));
                }
                5 => {
                    let list = self.variable_name();

                    lines.push(format!("{list} = []"));
                    lines.push(format!("{list}.push({current})"));
                    lines.push(format!("{current} = {list}[0]"));
                    description.push_str(&format!(
                        "creates a list {list}, adds {current} to {list}, and assigns {current} to the first element of {list}. "
                    ));
                }
                _ => unreachable!(),
            }
        }

        lines.push(format!("return {current}"));
        description.push_str(&format!("Finally, it returns {current}"));

        lines
    }

    pub(crate) fn arithmetic(
        &mut self,
        variable: &str,
        previous: Option<&str>,
    ) -> (String, String) {
        loop {
            let operator = OPERATORS[self.random.next_below(OPERATORS.len())];
            let number = 1 + self.random.next_below(9);
            let operation = format!("{variable} {operator} {number}");

            if previous != Some(&format!("{variable} = {operation}")) {
                let operator_description = match operator {
                    "+" => "plus",
                    "-" => "minus",
                    "*" => "multiplied by",
                    "/" => "divided by",
                    _ => unreachable!(),
                };

                return (
                    operation,
                    format!("{variable} {operator_description} {number}"),
                );
            }
        }
    }

    pub(crate) fn condition(&mut self, variable: &str) -> (String, String) {
        const COMPARISONS: [&str; 6] = ["==", "!=", "<", "<=", ">", ">="];
        const DESCRIPTIONS: [&str; 6] = [
            "equal to",
            "not equal to",
            "less than",
            "less than or equal to",
            "greater than",
            "greater than or equal to",
        ];
        let index = self.random.next_below(COMPARISONS.len());
        let number = 1 + self.random.next_below(9);

        (
            format!("{variable} {} {number}", COMPARISONS[index]),
            format!(
                "the value of {variable} is {} {number}",
                DESCRIPTIONS[index]
            ),
        )
    }

    pub(crate) fn function_call(&mut self, variable: &str) -> (String, String) {
        let function = self.function_name();
        let call_description = format!("calls the function {function} with {variable}");

        if !self
            .generated_functions
            .iter()
            .any(|(name, _)| name == &function)
        {
            let mut description = String::new();
            let lines = self.function_body(variable, &mut description, false);
            let mut code = format!("{function} = function({variable})\n");

            for line in lines {
                code.push_str("    ");
                code.push_str(&line);
                code.push('\n');
            }

            code.push_str("end function\n");
            self.generated_functions.push((function.clone(), code));
            self.additional_descriptions.push(format!(
                "The function {function} {}.",
                description.trim_end_matches('.')
            ));
        }

        (format!("{function}({variable})"), call_description)
    }

    pub(crate) fn introduce_errors(&mut self, code: &str) -> String {
        let mut lines = code.split('\n').map(str::to_owned).collect::<Vec<_>>();

        self.error_lines.clear();

        let count = 1 + self.random.next_below(5);
        let mut operation_indexes = BTreeSet::new();

        for _ in 0..count {
            let mut attempts = 0;
            let mut introduced = false;

            while !introduced && attempts < 10 {
                let operation = self.random.next_below(8);

                if operation_indexes.contains(&operation) {
                    attempts += 1;
                    continue;
                }

                let mut candidate = lines.clone();
                let mut candidate_errors = BTreeSet::new();

                self.apply_error(operation, &mut candidate, &mut candidate_errors);

                if normalized_code(&candidate.join("\n")) != normalized_code(&lines.join("\n")) {
                    lines = candidate;
                    self.error_lines.extend(candidate_errors);
                    operation_indexes.insert(operation);
                    introduced = true;
                } else {
                    attempts += 1;
                }
            }
        }

        if self.error_lines.is_empty() {
            let mut errors = BTreeSet::new();

            self.syntax_error(&mut lines, &mut errors);

            if errors.is_empty() {
                self.remove_line(&mut lines, &mut errors);
            }

            self.error_lines.extend(errors);
        }

        lines.join("\n")
    }

    pub(crate) fn apply_error(
        &mut self,
        operation: usize,
        lines: &mut Vec<String>,
        errors: &mut BTreeSet<usize>,
    ) {
        match operation {
            0 => self.remove_keyword(lines, errors),
            1 => self.remove_line(lines, errors),
            2 => self.syntax_error(lines, errors),
            3 => self.alter_loop(lines, errors),
            4 => self.change_operator(lines, errors),
            5 => self.incorrect_variable(lines, errors),
            6 => self.math_error(lines, errors),
            7 => self.change_number(lines, errors),
            _ => unreachable!(),
        }
    }

    pub(crate) fn syntax_error(&mut self, lines: &mut [String], errors: &mut BTreeSet<usize>) {
        let mut indexes = (0..lines.len()).collect::<Vec<_>>();

        self.shuffle(&mut indexes);

        for index in indexes {
            if lines[index].starts_with(&format!("{} = function", self.main_function)) {
                continue;
            }

            for token in KEYWORDS.into_iter().chain(["+", "-", "*", "/", "=", "=="]) {
                if lines[index].contains(token) {
                    lines[index] = lines[index].replace(token, &token[1..]);
                    errors.insert(index + 1);

                    return;
                }
            }
        }
    }

    pub(crate) fn remove_keyword(&mut self, lines: &mut [String], errors: &mut BTreeSet<usize>) {
        let candidates = (0..lines.len())
            .filter(|index| {
                !lines[*index].starts_with(&format!("{} = function", self.main_function))
                    && KEYWORDS
                        .iter()
                        .any(|keyword| lines[*index].contains(keyword))
            })
            .collect::<Vec<_>>();

        if candidates.is_empty() {
            return;
        }

        let index = candidates[self.random.next_below(candidates.len())];

        for keyword in KEYWORDS {
            if lines[index].contains(keyword) {
                lines[index] = lines[index].replace(keyword, "");
                errors.insert(index + 1);
                break;
            }
        }
    }

    pub(crate) fn remove_line(&mut self, lines: &mut Vec<String>, errors: &mut BTreeSet<usize>) {
        if self.line_removal_complete {
            return;
        }

        let start = self
            .error_lines
            .last()
            .map_or(1, |line| line.saturating_sub(1));
        let candidates = (start..lines.len().saturating_sub(1))
            .filter(|index| {
                !lines[*index].starts_with(&format!("{} = function", self.main_function))
                    && !lines[*index].is_empty()
                    && !self.error_lines.contains(&(*index + 1))
            })
            .collect::<Vec<_>>();

        if !candidates.is_empty() {
            let index = candidates[self.random.next_below(candidates.len())];

            errors.insert(index + 1);
            lines.remove(index);
            self.line_removal_complete = true;
        }
    }

    pub(crate) fn alter_loop(&mut self, lines: &mut [String], errors: &mut BTreeSet<usize>) {
        let mut indexes = (0..lines.len()).collect::<Vec<_>>();

        self.shuffle(&mut indexes);

        for index in indexes {
            if lines[index].starts_with(&format!("{} = function", self.main_function))
                || !lines[index].contains("for")
                || !lines[index].contains("in")
            {
                continue;
            }

            if let Some(start) = lines[index].find("range(")
                && let Some(end) = lines[index][start..].find(')')
            {
                let range = &lines[index][start..=start + end];
                let arguments = range
                    .trim_start_matches("range(")
                    .trim_end_matches(')')
                    .split(',')
                    .map(str::trim)
                    .collect::<Vec<_>>();

                if arguments.len() == 2
                    && arguments[0] == "0"
                    && let Ok(number) = arguments[1].parse::<usize>()
                {
                    lines[index] =
                        lines[index].replace(range, &format!("range(0, {})", number + 1));
                    errors.insert(index + 1);
                    break;
                }
            }
        }
    }

    pub(crate) fn change_operator(&mut self, lines: &mut [String], errors: &mut BTreeSet<usize>) {
        let mut indexes = (0..lines.len()).collect::<Vec<_>>();

        self.shuffle(&mut indexes);

        for index in indexes {
            if lines[index].starts_with(&format!("{} = function", self.main_function)) {
                continue;
            }

            for (operator, replacement) in OPERATORS.into_iter().zip(REPLACEMENT_OPERATORS) {
                if lines[index].contains(operator) && !self.error_lines.contains(&(index + 1)) {
                    lines[index] = lines[index].replace(operator, replacement);
                    errors.insert(index + 1);

                    return;
                }
            }
        }
    }

    pub(crate) fn incorrect_variable(
        &mut self,
        lines: &mut [String],
        errors: &mut BTreeSet<usize>,
    ) {
        if lines.len() < 2 {
            return;
        }

        let replacement = self.variable_name();
        let variable = lines.iter().find_map(|line| {
            split_tokens(line).into_iter().find(|token| {
                token.starts_with("var_")
                    || (token.starts_with("func_") && !token.starts_with(&self.main_function))
            })
        });
        let Some(variable) = variable else {
            return;
        };

        for (index, line) in lines.iter_mut().enumerate() {
            if !line.starts_with(&format!("{} = function", self.main_function))
                && line.contains(&variable)
            {
                *line = line.replace(&variable, &replacement);
                errors.insert(index + 1);
                break;
            }
        }
    }

    pub(crate) fn math_error(&mut self, lines: &mut [String], errors: &mut BTreeSet<usize>) {
        let mut indexes = (0..lines.len()).collect::<Vec<_>>();

        self.shuffle(&mut indexes);

        for index in indexes {
            if lines[index].starts_with(&format!("{} = function", self.main_function))
                || !lines[index].contains('=')
                || !OPERATORS
                    .iter()
                    .any(|operator| lines[index].contains(operator))
            {
                continue;
            }

            for (operator, replacement) in OPERATORS.into_iter().zip(REPLACEMENT_OPERATORS) {
                if lines[index].contains(operator) && !self.error_lines.contains(&(index + 1)) {
                    lines[index] = lines[index].replace(operator, replacement);
                    errors.insert(index + 1);

                    return;
                }
            }
        }
    }

    pub(crate) fn change_number(&mut self, lines: &mut [String], errors: &mut BTreeSet<usize>) {
        let mut indexes = (0..lines.len()).collect::<Vec<_>>();

        self.shuffle(&mut indexes);

        for index in indexes {
            if lines[index].starts_with(&format!("{} = function", self.main_function))
                || (!lines[index].contains("return") && !lines[index].contains('='))
            {
                continue;
            }

            if let Some(number) = split_tokens(&lines[index])
                .into_iter()
                .find(|token| token.parse::<i32>().is_ok())
            {
                let replacement = 1 + self.random.next_below(9);

                if number != replacement.to_string() {
                    lines[index] = lines[index].replace(&number, &replacement.to_string());
                    errors.insert(index + 1);
                    break;
                }
            }
        }
    }

    pub(crate) fn shuffle<T>(&mut self, values: &mut [T]) {
        let mut count = values.len();

        while count > 1 {
            count -= 1;

            let index = self.random.next_below(count + 1);

            values.swap(index, count);
        }
    }

    pub(crate) fn function_name(&mut self) -> String {
        format!("func_{}", self.random_string(6))
    }

    pub(crate) fn variable_name(&mut self) -> String {
        format!("var_{}", self.random_string(6))
    }

    pub(crate) fn random_string(&mut self, length: usize) -> String {
        (0..length)
            .map(|_| ALPHANUMERIC[self.random.next_below(ALPHANUMERIC.len())] as char)
            .collect()
    }
}

fn split_tokens(line: &str) -> Vec<String> {
    line.split([
        ' ', '(', ')', '[', ']', '+', '-', '*', '/', '=', '%', ',', ':', '.',
    ])
    .filter(|token| !token.is_empty())
    .map(str::to_owned)
    .collect()
}

pub(crate) fn normalized_code(code: &str) -> String {
    code.chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}
