

#[derive(Debug, PartialEq)]
pub enum ParseOp {
    And,
    Or,
    Seq,
}

// TODO: refactor, with Success(T, &str)
#[derive(Debug, PartialEq)]
pub enum ParseRes<T> {
    Success(T),
    Incomplete,
    Invalid(String),
}

#[derive(Debug, PartialEq)]
pub enum Parsed {
    Sentence(Vec<String>),
    Expr(Box<Parsed>, Box<Parsed>, ParseOp),
}

#[derive(Debug, PartialEq)]
enum DQToken {
    Str(String),
    Exp(ExpToken),
}

#[derive(Debug, PartialEq)]
enum ExpToken {
    Command(Vec<Token>),
    Param(String),
    Arith(String)
}

#[derive(Debug, PartialEq)]
enum Token {
    Unquoted(String),
    Expansion(ExpToken),
    DoubleQuote(Vec<DQToken>),
    SingleQuote(String),
    PathExp,
    TildeExp,
    Space
}

/// Try macro for ParseRes
macro_rules! try {
    ($expr:expr) => (match $expr {
        ParseRes::Success(val) => val,
        ParseRes::Invalid(err) => return ParseRes::Invalid(err),
        ParseRes::Incomplete => return ParseRes::Incomplete,
    });
}


pub fn parse_command(text: &str) -> ParseRes<Parsed> {
    let ast = try!(parse_unquoted(text, String::new(), Vec::new(), false)).0;
    process_tokens(ast)
}

fn process_tokens(ast: Vec<Token>) -> ParseRes<Parsed> {
    let ast = try!(expr_expansion(ast));
    let ast = try!(field_splitting(ast));
    let ast = try!(pathname_expansion(ast));
    to_parsed_form(ast)
}

fn process_and_exec(ast: Vec<Token>) -> ParseRes<String> {
    let parsed = try!(process_tokens(ast));
    ParseRes::Invalid("Unimplemented".to_owned())
}

fn expand_param(param: String) -> ParseRes<String> {
    ParseRes::Invalid("Unimplemented".to_owned())
}

fn expand_arith(expr: String) -> ParseRes<String> {
    ParseRes::Invalid("Unimplemented".to_owned())
}

fn expr_expansion(ast: Vec<Token>) -> ParseRes<Vec<Token>> {
    // TODO: tilde expansion
    let mut vec = Vec::new();

    for token in ast {
        match token {
            Token::Expansion(exp_tok) => {
                let str_res = match exp_tok {
                    ExpToken::Command(tokens) => try!(process_and_exec(tokens)),
                    ExpToken::Param(param) => try!(expand_param(param)),
                    ExpToken::Arith(expr) => try!(expand_arith(expr)),
                };
                vec.push(Token::Unquoted(str_res));
            },
            _ => vec.push(token),
        }
    }
    /*
    for ref mut token in &mut ast {
        if let &mut &mut Token::Expansion(ref exp_tok) = token {
            let exp_tok: ExpToken = (*exp_tok).clone();

            let str_res = match exp_tok {
                ExpToken::Command(tokens) => try!(process_and_exec(tokens)),
                ExpToken::Param(param) => try!(expand_param(param)),
                ExpToken::Arith(expr) => try!(expand_arith(expr)),
            };
            **token = Token::Unquoted(str_res);
        }
    }*/

    ParseRes::Success(vec)
}

fn field_splitting(ast: Vec<Token>) -> ParseRes<Vec<Vec<Token>>> {
    use self::Token::*;

    let mut result = Vec::new();
    let mut current = Vec::new();

    let mut s = String::new();

    for token in ast {
        match token {
            Unquoted(string) => {
                for c in string.chars() {
                    if c == ' ' {
                        if !s.is_empty() {
                            current.push(Unquoted(s));
                            result.push(current);
                            current = Vec::new();
                            s = String::new();
                        }
                    } else {
                        s.push(c);
                    }
                }
            },
            token => {
                current.push(Unquoted(s));
                s = String::new();
                current.push(token);
            },
        }
    }

    if !s.is_empty() {
        current.push(Unquoted(s));
    }

    if !current.is_empty() {
        result.push(current);
    }

    ParseRes::Success(result)
}

fn pathname_expansion(ast: Vec<Vec<Token>>) -> ParseRes<Vec<Vec<Token>>> {
    ParseRes::Success(ast)
}

fn to_parsed_form(ast: Vec<Vec<Token>>) -> ParseRes<Parsed> {
    use self::Token::*;
    let error_msg = "Error converting to parsed form";
    let mut sentence = Vec::new();

    for word in ast {
        let mut current = String::new();
        for token in word {
            match token {
                Unquoted(string) => current.push_str(string.as_str()),
                DoubleQuote(tokens) => {
                    for token in tokens {
                        match token {
                            DQToken::Str(string) => current.push_str(string.as_str()),
                            DQToken::Exp(_) => return ParseRes::Invalid(error_msg.to_owned()),
                        }
                    }
                },
                SingleQuote(string) => current.push_str(string.as_str()),
                Space => current.push(' '),
                _ => return ParseRes::Invalid(error_msg.to_owned()),
            }
        }
        sentence.push(current);
    }

    ParseRes::Success(Parsed::Sentence(sentence))
}

// TODO: DRY
fn parse_unquoted(text: &str, mut curr_expr: String, mut tokens: Vec<Token>, sub: bool)
                  -> ParseRes<(Vec<Token>, &str)> {
    use self::Token::*;
    let mut chars = text.chars();

    match chars.next() {
        Some(next_char) => match next_char {
            '\n' => {
                if curr_expr.len() > 0 { tokens.push(Unquoted(curr_expr)); }
                ParseRes::Success( (tokens, &text[1..]) )
            },
            '\'' => {
                if curr_expr.len() > 0 { tokens.push(Unquoted(curr_expr)); }
                let (result, rest) = try!(parse_single_quoted_expr(&text[1..], String::new()));
                tokens.push(result);

                parse_unquoted(rest, String::new(), tokens, sub)
            },
            '"' => {
                if curr_expr.len() > 0 { tokens.push(Unquoted(curr_expr)); }
                let (result, rest) = try!(parse_double_quoted_expr(
                        &text[1..], String::new(), Vec::new()));
                tokens.push(result);

                parse_unquoted(rest, String::new(), tokens, sub)
            },
            '$' => {
                if curr_expr.len() > 0 { tokens.push(Unquoted(curr_expr)); }
                let (result, rest) = try!(parse_dollar_expr(&text[1..]));
                tokens.push(result);

                parse_unquoted(rest, String::new(), tokens, sub)
            },
            '#' => {
                if curr_expr.len() > 0 { tokens.push(Unquoted(curr_expr)); }
                let rest = parse_comment(&text[1..]);
                parse_unquoted(rest, String::new(), tokens, sub)
            },
            '\\' => match chars.next() {
                Some(next_char) => match next_char {
                    ' ' => {
                        if curr_expr.len() > 0 { tokens.push(Unquoted(curr_expr)); }
                        tokens.push(Space);
                        parse_unquoted(&text[2..], String::new(), tokens, sub)
                    },
                    '\n' => parse_unquoted(&text[2..], curr_expr, tokens, sub),
                    c => {
                        curr_expr.push(c);
                        parse_unquoted(&text[2..], curr_expr, tokens, sub)
                    },
                },
                None => ParseRes::Incomplete,
            },
            '*' => {
                if curr_expr.len() > 0 { tokens.push(Unquoted(curr_expr)); }
                tokens.push(PathExp);
                parse_unquoted(&text[1..], String::new(), tokens, sub)
            },
            '~' => {
                if curr_expr.len() > 0 { tokens.push(Unquoted(curr_expr)); }
                tokens.push(TildeExp);
                parse_unquoted(&text[1..], String::new(), tokens, sub)
            },
            ')' => {
                if sub {
                    if curr_expr.len() > 0 { tokens.push(Unquoted(curr_expr)); }
                    ParseRes::Success( (tokens, &text[1..]) )
                } else { ParseRes::Invalid("Unexpected ')'".to_owned()) }
            },
            c => {
                curr_expr.push(c);
                parse_unquoted(&text[1..], curr_expr, tokens, sub)
            },
        },
        None => ParseRes::Incomplete,
    }
}

fn parse_comment(text: &str) -> &str {
    match text.chars().next() {
        Some(c) => match c {
            '\n' => text,
            _ => parse_comment(&text[1..]),
        },
        None => text,
    }
}

fn parse_single_quoted_expr(text: &str, mut curr_expr: String) -> ParseRes<(Token, &str)> {
    match text.chars().next() {
        Some(c) => match c {
            '\'' => ParseRes::Success( (Token::SingleQuote(curr_expr), &text[1..]) ),
            c => {
                curr_expr.push(c);
                parse_single_quoted_expr(&text[1..], curr_expr)
            },
        },
        None => ParseRes::Incomplete,
    }
}

fn parse_double_quoted_expr(text: &str, mut curr_expr: String, mut dq_tokens: Vec<DQToken>)
                            -> ParseRes<(Token, &str)> {
    let mut chars = text.chars();

    match chars.next() {
        Some(c) => match c {
            '\\' => {
                match chars.next() {
                    Some(c) => {
                        match c {
                            '\\' => curr_expr.push('\\'),
                            '\n' => { },
                            '"' => curr_expr.push('"'),
                            '$' => curr_expr.push('$'),
                            '`' => curr_expr.push('`'),
                            c => {
                                curr_expr.push('\\');
                                curr_expr.push(c)
                            },
                        };
                        parse_double_quoted_expr(&text[2..], curr_expr, dq_tokens)
                    },
                    None => ParseRes::Incomplete,
                }
            },
            '"' => {
                if curr_expr.len() > 0 { dq_tokens.push(DQToken::Str(curr_expr)); }
                ParseRes::Success( (Token::DoubleQuote(dq_tokens), &text[1..]) )
            },
            '$' => {
                if curr_expr.len() > 0 { dq_tokens.push(DQToken::Str(curr_expr)); }

                let (token, rest) = try!(parse_dollar_expr(&text[1..]));
                dq_tokens.push(
                    match token {
                        Token::Expansion(exp) => DQToken::Exp(exp),
                        Token::Unquoted(string) => DQToken::Str(string),
                        _ => return ParseRes::Invalid("TODO: improve msg".to_owned()),
                    }
                );
                parse_double_quoted_expr(rest, String::new(), dq_tokens)
            },
            c => {
                curr_expr.push(c);
                parse_double_quoted_expr(&text[1..], curr_expr, dq_tokens)
            },
        },
        None => ParseRes::Incomplete,
    }

}



fn parse_dollar_expr(text: &str) -> ParseRes<(Token, &str)> {
    match text.chars().next() {
        Some(c) => match c {
            '{' => {
                parse_bracketed_param(&text[1..], String::new())
            },
            '(' => {
                parse_dollar_paren_expr(&text[1..])
            },
            '\n' => ParseRes::Success( (Token::Unquoted("$".to_owned()), &text[1..]) ),
            _ => parse_unbracketed_param(text),
        }
        None => ParseRes::Incomplete,
    }
}

fn parse_unbracketed_param(text: &str) -> ParseRes<(Token, &str)> {
    ParseRes::Invalid("Parameters not yet supported.".to_owned())
}

fn parse_dollar_paren_expr(text: &str) -> ParseRes<(Token, &str)> {
    match text.chars().next() {
        Some(c) => match c {
            '(' => parse_arith_expr(&text[1..]),
            _ => parse_subcommand(text),
        },
        None => ParseRes::Incomplete,
    }
}

fn parse_arith_expr(text: &str) -> ParseRes<(Token, &str)> {
    ParseRes::Invalid("Arithmetic expressions not yet supported.".to_owned())
}

fn parse_subcommand(text: &str) -> ParseRes<(Token, &str)> {
    let (tokens, rest) = try!(parse_unquoted(text, String::new(), Vec::new(), true));

    ParseRes::Success( (Token::Expansion(ExpToken::Command(tokens)), rest) )
}

fn parse_bracketed_param(text: &str, mut curr_expr: String) -> ParseRes<(Token, &str)> {
    ParseRes::Invalid("Parameters not yet supported.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! make_assert_success {
        ($input_type: ty, $pattern:pat, $func:expr,
         $({$exp_name:ident: $exp_type:ty; $res_id:ident}),*) => {
            fn assert_success(input: $input_type, $($exp_name: $exp_type),*) {
                let result = $func(input);
                if let $pattern = result {
                    $(
                        assert_eq!($res_id, $exp_name);
                    )*
                } else {
                    panic!("Result doesn't match type.");
                }
            }
        }
    }

    #[test]
    fn test_parse_command() {
        make_assert_success!(&str, ParseRes::Success(Parsed::Sentence(result)),
                             |x| parse_command(x), {expected: Vec<String>; result});
        assert_success("ls --help\n", vec!["ls".to_owned(), "--help".to_owned()]);

        assert_success("echo \"abc 123\"\n",
                        vec!["echo".to_owned(), "abc 123".to_owned()]);
    }


    #[test]
    fn test_parse_unquoted() {
        use super::Token::*;

        make_assert_success!(&str, ParseRes::Success( (tokens, text) ),
                             |x| parse_unquoted(x, String::new(), Vec::new(), false),
                             {ast: Vec<Token>; tokens}
                            );
        assert_success(
            "ls $(pwd)\n",
            vec![Unquoted("ls ".to_owned()),
                 Expansion(ExpToken::Command(vec![Unquoted("pwd".to_owned())]))]
        );
        assert_success(
            "echo ab\\ c\"$(ls './file')\"\n",
            vec![Unquoted("echo ab".to_owned()), Space, Unquoted("c".to_owned()),
                 DoubleQuote(vec![DQToken::Exp(ExpToken::Command(vec![
                     Unquoted("ls ".to_owned()), SingleQuote("./file".to_owned())
                 ]))])]
        );
    }

}
