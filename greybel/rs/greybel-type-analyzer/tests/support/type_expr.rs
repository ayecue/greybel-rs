use greybel_type_analyzer::TypeExpr;

pub fn round_trip(text: &str) -> Option<String> {
    TypeExpr::parse(text).map(|expr| expr.to_string())
}
