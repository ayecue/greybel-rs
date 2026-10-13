use std::rc::Rc;
use std::time::Duration;

use greybel_interpreter::{Clock, Pin, PrintOptions, Value, ValueType, Vm, native};
use mono_random::Random;

const MINISCRIPT_VERSION: &str = "1.6.2";
const CURRENT_PROGRAM: &str = "(current program)";

#[derive(Debug, Clone)]
pub struct HostInfo {
    pub name: String,
    pub repository_url: String,
    pub version: f64,
    pub build_date: String,
}

impl Default for HostInfo {
    fn default() -> Self {
        Self {
            name: env!("CARGO_PKG_NAME").to_owned(),
            repository_url: env!("CARGO_PKG_REPOSITORY").to_owned(),
            version: concat!(
                env!("CARGO_PKG_VERSION_MAJOR"),
                ".",
                env!("CARGO_PKG_VERSION_MINOR")
            )
            .parse()
            .unwrap_or_default(),
            build_date: String::new(),
        }
    }
}

#[derive(Default)]
struct HostData {
    version: Option<Pin>,
    intrinsics: Option<Pin>,
}

fn host_data_with_cache(
    vm: &mut Vm,
    select: fn(&mut HostData) -> &mut Option<Pin>,
    create: fn(&mut Vm) -> Value,
) -> Value {
    let cache_entry = vm
        .host_data_mut::<HostData>()
        .and_then(|cache| select(cache).take());
    let pin = match cache_entry {
        Some(pin) => pin,
        None => {
            let value = create(vm);

            vm.pin(value)
        }
    };
    let value = vm.pinned_value(&pin).clone();

    *select(vm.host_data_or_insert_with(HostData::default)) = Some(pin);

    value
}

#[native]
pub fn print(vm: &mut Vm, #[default("")] s: &Value, delimiter: &Value) {
    let heap = vm.heap();
    let mut output = heap.display(s).to_string();
    let new_line = delimiter.is_null();

    if !new_line {
        output.push_str(&heap.display(delimiter).to_string());
    }

    let options = PrintOptions {
        new_line,
        ..PrintOptions::default()
    };

    vm.stdout().print(&output, options);
}

#[native]
pub fn time(vm: &mut Vm) -> f64 {
    vm.run_time()
}

#[native]
pub async fn wait(#[host] clock: Rc<dyn Clock>, #[default(1)] seconds: f64) {
    clock
        .sleep(Duration::try_from_secs_f64(seconds).unwrap_or_default())
        .await;
}

#[native]
pub fn end_time_slice(vm: &mut Vm) {
    vm.request_yield();
}

#[native]
pub fn stack_trace(vm: &mut Vm) -> Vec<String> {
    vm.stack_trace()
        .into_iter()
        .map(|location| {
            let file = if location.path.is_empty() {
                CURRENT_PROGRAM
            } else {
                &location.path
            };

            format!("{file} line {}", location.line)
        })
        .collect()
}

#[native]
pub fn version(vm: &mut Vm) -> Value {
    host_data_with_cache(
        vm,
        |cache| &mut cache.version,
        |vm| {
            let host = vm.host_data::<HostInfo>().cloned().unwrap_or_default();
            let entries = [
                ("miniscript", Value::from(MINISCRIPT_VERSION)),
                ("buildDate", Value::from(host.build_date)),
                ("host", Value::from(host.version)),
                ("hostName", Value::from(host.name)),
                ("hostInfo", Value::from(host.repository_url)),
            ];
            let map = vm.heap_mut().add_map_from(
                entries
                    .into_iter()
                    .map(|(key, value)| (Value::from(key), value)),
            );

            Value::Map(map)
        },
    )
}

#[native]
pub fn intrinsics(vm: &mut Vm) -> Value {
    host_data_with_cache(
        vm,
        |cache| &mut cache.intrinsics,
        |vm| {
            let entries: Vec<(Value, Value)> = vm
                .api()
                .entries(vm.heap())
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();

            Value::Map(vm.heap_mut().add_map_from(entries))
        },
    )
}

fn type_map(vm: &mut Vm, kind: ValueType) -> Value {
    Value::Map(vm.type_map(kind))
}

#[native]
pub fn func_ref(vm: &mut Vm) -> Value {
    type_map(vm, ValueType::Function)
}

#[native]
pub fn list(vm: &mut Vm) -> Value {
    type_map(vm, ValueType::List)
}

#[native]
pub fn map(vm: &mut Vm) -> Value {
    type_map(vm, ValueType::Map)
}

#[native]
pub fn number(vm: &mut Vm) -> Value {
    type_map(vm, ValueType::Number)
}

#[native]
pub fn string(vm: &mut Vm) -> Value {
    type_map(vm, ValueType::String)
}

#[native]
pub fn rnd(vm: &mut Vm, seed: &Value) -> f64 {
    if !seed.is_null() {
        vm.set_host_data(Random::new(seed.to_int()));
    }

    vm.host_data_or_insert_with(Random::from_entropy)
        .next_double()
}

#[native]
pub fn hash(vm: &mut Vm, obj: &Value) -> f64 {
    f64::from(vm.heap().hash_code(obj))
}

#[native]
pub fn ref_equals(vm: &mut Vm, a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::String(a), Value::String(b)) => a.ptr_eq(b),
        (Value::List(a), Value::List(b)) => a == b,
        (Value::Map(a), Value::Map(b)) => a == b,
        (Value::Function(_), Value::Function(_)) => vm.heap().equal(a, b),
        _ => false,
    }
}
