use woothee::parser::Parser;

#[derive(Debug, Clone, Default)]
pub struct ParsedUA {
    pub browser: Option<String>,
    pub os: Option<String>,
    pub device: Option<String>,
}

pub fn parse_user_agent(ua: &str) -> ParsedUA {
    let parser = Parser::new();
    match parser.parse(ua) {
        Some(result) => ParsedUA {
            browser: Some(result.name.to_string()),
            os: Some(result.os.to_string()),
            device: Some(match result.category.as_ref() {
                "smartphone" => "mobile",
                "mobilephone" => "mobile",
                "tablet" => "tablet",
                _ => "desktop",
            }.to_string()),
        },
        None => ParsedUA::default(),
    }
}
