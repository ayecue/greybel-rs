/// Definition exercising every supported tag location.
pub const DEFINITION: &str = r#"use core 1.0;
tag origin: string on protocol, message;
tag game_id: u32 on message;
tag list: string on field;
tag window on field;
tag alias: string on field, variant, member repeated;
tag kind: Kind on struct, union, enum;

@origin("game")
protocol P = "p" {
    client C {}
    provider D {
        @game_id(34) @origin("extension")
        message Scan { @list("strValues") ip: string, @list("intValues") @window pid: i32 }
    }
}

@kind(Value)
struct Point { @alias("x") @alias("across") x: f32 }
@kind(Sum)
union Shape { @alias("dot") Dot, Line { @window from: i32 } }
@kind(Value)
enum Kind { @alias("plain") Value, Sum }
"#;
