#[derive(Debug, Clone)]
pub struct TelegramParsedCommand {
    pub command: String,
    pub args: Vec<String>,
}

pub fn parse_command(input: &str) -> Option<TelegramParsedCommand> {
    let cleaned = input.trim();
    if cleaned.is_empty() {
        return None;
    }

    let mut tokens = cleaned.split_whitespace();
    let raw_command = tokens.next()?.trim();
    let command = raw_command
        .split('@')
        .next()
        .unwrap_or(raw_command)
        .to_lowercase();

    let args = tokens.map(ToString::to_string).collect::<Vec<_>>();
    Some(TelegramParsedCommand { command, args })
}

pub fn parse_amount(text: &str) -> Option<i64> {
    let normalized = text
        .trim()
        .replace(',', "")
        .replace('원', "")
        .replace('+', "");
    normalized.parse::<i64>().ok().filter(|v| *v > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_parses_with_bot_suffix() {
        let parsed = parse_command("/today@ledger_bot   추가값").unwrap();
        assert_eq!(parsed.command, "/today");
        assert_eq!(parsed.args, vec!["추가값"]);
    }

    #[test]
    fn amount_parses_korean_won_format() {
        assert_eq!(parse_amount("12,300원"), Some(12300));
        assert_eq!(parse_amount("-1000"), None);
    }
}
