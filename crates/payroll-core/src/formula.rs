use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use crate::Decimal;

#[derive(Clone, Debug, PartialEq)]
enum Token { Number(Decimal), Name(String), Plus, Minus, Times, Divide, Left, Right }

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut out = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() { i += 1; continue; }
        let operator = match c { '+' => Some(Token::Plus), '-' => Some(Token::Minus), '*' => Some(Token::Times), '/' => Some(Token::Divide), '(' => Some(Token::Left), ')' => Some(Token::Right), _ => None };
        if let Some(op) = operator { out.push(op); i += 1; continue; }
        if c.is_ascii_digit() || c == '.' {
            let start = i; i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') { i += 1; }
            let value: String = chars[start..i].iter().collect();
            out.push(Token::Number(Decimal::from_str(&value).map_err(|_| format!("无效数字：{value}"))?));
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i; i += 1;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') { i += 1; }
            out.push(Token::Name(chars[start..i].iter().collect()));
            continue;
        }
        return Err(format!("公式含有不允许的字符：{c}"));
    }
    if out.len() > 256 { return Err("公式过长".into()); }
    Ok(out)
}

pub fn references(input: &str) -> Result<BTreeSet<String>, String> {
    let tokens = tokenize(input)?;
    let mut names = BTreeSet::new();
    for token in tokens { if let Token::Name(name) = token { names.insert(name); } }
    Ok(names)
}

struct Parser<'a> { tokens: &'a [Token], at: usize, values: &'a BTreeMap<String, Decimal> }
impl Parser<'_> {
    fn take(&mut self, expected: &Token) -> bool { if self.tokens.get(self.at) == Some(expected) { self.at += 1; true } else { false } }
    fn expression(&mut self) -> Result<Decimal, String> {
        let mut value = self.term()?;
        loop {
            if self.take(&Token::Plus) { value = value.checked_add(self.term()?).ok_or("金额溢出")?; }
            else if self.take(&Token::Minus) { value = value.checked_sub(self.term()?).ok_or("金额溢出")?; }
            else { return Ok(value); }
        }
    }
    fn term(&mut self) -> Result<Decimal, String> {
        let mut value = self.factor()?;
        loop {
            if self.take(&Token::Times) { value = value.checked_mul(self.factor()?).ok_or("金额溢出")?; }
            else if self.take(&Token::Divide) {
                let divisor = self.factor()?;
                if divisor.is_zero() { return Err("不能除以零".into()); }
                value = value.checked_div(divisor).ok_or("金额溢出")?;
            } else { return Ok(value); }
        }
    }
    fn factor(&mut self) -> Result<Decimal, String> {
        if self.take(&Token::Minus) { return Ok(-self.factor()?); }
        if self.take(&Token::Plus) { return self.factor(); }
        if self.take(&Token::Left) {
            let value = self.expression()?;
            if !self.take(&Token::Right) { return Err("缺少右括号".into()); }
            return Ok(value);
        }
        let token = self.tokens.get(self.at).ok_or("公式不完整")?;
        self.at += 1;
        match token {
            Token::Number(value) => Ok(*value),
            Token::Name(name) => self.values.get(name).copied().ok_or_else(|| format!("缺少字段：{name}")),
            _ => Err("公式语法错误".into()),
        }
    }
}

pub fn evaluate(input: &str, values: &BTreeMap<String, Decimal>) -> Result<Decimal, String> {
    let tokens = tokenize(input)?;
    let mut parser = Parser { tokens: &tokens, at: 0, values };
    let value = parser.expression()?;
    if parser.at != tokens.len() { return Err("公式尾部有多余内容".into()); }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn precedence_and_restricted_syntax() {
        let fields = BTreeMap::from([("base".into(), Decimal::from(10000))]);
        assert_eq!(evaluate("base / (2 + 3) * 2", &fields).unwrap(), Decimal::from(4000));
        assert!(evaluate("base; system('rm')", &fields).is_err());
        assert!(evaluate("base / 0", &fields).is_err());
        assert!(evaluate("base + missing", &fields).is_err());
    }
}
