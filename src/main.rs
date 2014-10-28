use std::collections::HashMap;

use std::iter::RangeInclusive;
use std::iter::range_inclusive;

trait IsAlpha {
    fn is_alpha(self) -> bool;
}

impl IsAlpha for char {
    fn is_alpha(self) -> bool {
        (self >= 'a' && self <= 'z') || (self >= 'A' && self <= 'Z')
    }    
}

struct StringReader {
    pos:    uint,
    source: String,
    col:    uint,
    line:   uint
}

impl StringReader {
    fn new(source: String) -> StringReader {
        StringReader {
            pos: 0,
            line: 1,
            col: 0,
            source: source
        }
    }

    fn peak(&mut self) -> Option<char> {
        if self.pos < self.source.len() {
            Some(self.source.as_slice().char_at(self.pos))
        } else {
            None
        }
    }

    fn read(&mut self) -> Option<char> {
        let next = self.peak();
        if next.is_some()  {
            if next.unwrap() == '\n' {
                self.line += 1;
                self.col = 0;
            } else {
                self.col += 1;
            }
            self.pos += 1;
        }
        next
    }
}

struct ErrorPosition {
    msg:        String,
    line:       uint,
    col_range:  RangeInclusive<uint>
}

struct Lexer {
    reader: StringReader
}

impl Lexer {
    fn tok(&self, token_type: Type) -> Token {
        Token { token_type: token_type, col: self.reader.col, line: self.reader.line }
    }

    fn next_token(&mut self) -> Result<Token, ErrorPosition> {
        loop {
            let c = match self.reader.read() {
                Some(c) => c,
                None => return Ok(self.tok(EOF))
            };

            if      c == '(' { return Ok(self.tok(LParen)) }
            else if c == ')' { return Ok(self.tok(RParen)) }
            else if c == '&' || c == '*' { return Ok(self.tok(And)) }
            else if c == '|' || c == '+' { return Ok(self.tok(Or)) }
            else if c == '!' || c == '~' { return Ok(self.tok(Not)) }
            else if c == '^' { return Ok(self.tok(Xor)) }

            else if c.is_alpha() { return Ok(self.next_ident(c)) }

            else if c == ' ' || c == '\n' { continue }
            else {
                return Err(ErrorPosition {
                    msg:        format!("Unexpected character: {}", c).to_string(),
                    line:       self.reader.line,
                    col_range:  range_inclusive(self.reader.col, self.reader.col)    
                })
                
            }
        }
    }

    fn next_ident(&mut self, current: char) -> Token {
        let mut string = String::new();
        string.push(current);

        loop {
            let peak = self.reader.peak();
            if peak.is_some() && peak.unwrap().is_alpha() {
                string.push(peak.unwrap());
                self.reader.read();
            } else {
                break
            }
        }
        
        self.tok(Ident(string))
    }
}

#[deriving(Show)]
#[deriving(Clone)]
enum Type {
    LParen,
    RParen,

    Ident(String),

    And,
    Or,
    Not,
    Xor,

    EOF
}

#[deriving(Clone)]
#[deriving(Show)]
struct Token {
    token_type: Type,
    col:        uint,
    line:       uint
}

#[deriving(Show)]
struct Operation {
    components: Vec<Component>,
    ops: Vec<Token>
}

impl Operation {
    fn eval(&self, env: &Environment) -> Result<bool, ErrorPosition> {
        let mut val = false;
        if self.components.len() > 0 {
            val = try!(self.components[0].eval(env));
        }

        for idx in range(1u, self.components.len()) {
            let eval = try!(self.components[idx].eval(env));
            //let token = self.ops[idx - 1];
            match self.ops[idx - 1].token_type {
                And => val &= eval,
                Or => val |= eval,
                Xor => val ^= eval,
                ref other => {
                    return Err(ErrorPosition {
                        msg:        format!("Unexpected operation: {}", other).to_string(),
                        line:       self.ops[idx - 1].line,
                        col_range:  range_inclusive(self.ops[idx - 1].col, self.ops[idx - 1].col)
                    })
                }
            };
        }

        Ok(val)
    }

    fn get_variables(&self) -> Vec<String> {
        let mut vars: Vec<String> = Vec::new();

        for component in self.components.iter() {
            match component.value {
                Var(ref var) => {
                    if !vars.contains(var) { vars.push(var.clone()) }
                }
                Expr(ref op) => {
                    let other_vars = op.get_variables();
                    for var in other_vars.iter() {
                        if !vars.contains(var) { vars.push(var.clone()) }
                    }
                }
            }
        }

        vars
    }

    // TODO make structure to clean up this return type
    fn truth_table(&self) -> Result<Vec<(HashMap<String, bool>, bool)>, ErrorPosition> {
        let mut result = Vec::new();

        let vars = self.get_variables();
        let tests = std::num::pow(2i, vars.len());

        for num in range(0i, tests) {
            let mut env = EnvironmentImpl { vars: HashMap::new() };
            for pos in range(0u, vars.len()) {
                env.vars.insert(vars[pos].clone(), ((num >> (vars.len() - 1 - pos)) & 1) == 1);
            }
            result.push((env.vars.clone(), try!(self.eval(&env))));
        }

        Ok(result)
    }
}

#[deriving(Show)]
enum VarOrExpr {
    Var(String),
    Expr(Operation)
}

#[deriving(Show)]
struct Component {
    value: VarOrExpr,
    negated: bool
}

impl Component {
    fn eval(&self, env: &Environment) -> Result<bool, ErrorPosition> {
        let mut val = match self.value {
            Var(ref name) => env.get_variable(name.clone()),
            Expr(ref op) => try!(op.eval(env))
        };
        if self.negated { val = !val };
        Ok(val)
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: uint
}

impl Parser {
    fn new(lexer: &mut Lexer) -> Result<Parser, ErrorPosition> {
        let mut tokens = vec!();
        let mut token;
        loop {
            token = try!(lexer.next_token());
            match token.token_type {
                EOF => {
                    tokens.push(token);
                    break
                },
                _   => {
                    tokens.push(token);
                }
            }
        }
        Ok(Parser { tokens: tokens, pos: 0 })
    }
    
    fn next(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        self.pos += 1;
        tok
    }
    
    fn back(&mut self) { self.pos -= 1; }
    
    fn parse(&mut self) -> Operation {
        let mut op = Operation { components: vec!(), ops: vec!() };
        
        op.components.push(self.component());
        let mut token = self.next();

        loop {
            match token.token_type {
                Or | Xor | And => {
                    op.ops.push(token.clone());
                    op.components.push(self.component());
                },
                _ => {
                    break;
                }
            };
            token = self.next();
        }
        self.back();
        
        op
    }
    
    fn component(&mut self) -> Component {
        let mut token = self.next();
        let mut neg = false;
        let mut val: VarOrExpr;
        
        loop {
            match token.token_type {
                Not => neg = !neg,
                LParen => {
                    val = Expr(self.parse());
                    match self.next().token_type {
                        RParen  => {},
                        other   => { fail!("Unexpected token: {}", other); }
                    };
                    break;
                },
                Ident(name) => {
                    val = Var(name);
                    break;
                },
                other => { fail!("Unexpected token: {}", other); }
            }
            token = self.next();
        }

        Component { value: val, negated: neg }
    }
    
}

trait Environment {
    fn get_variable(&self, name: String) -> bool;
}

struct EnvironmentImpl {
    vars: HashMap<String, bool>
}

impl Environment for EnvironmentImpl {
    fn get_variable(&self, name: String) -> bool {
        match self.vars.find(&name) {
            Some(var) => *var,
            None => false
        }
    }
}

#[allow(unused_variable)]
fn repeat_char(c: char, times: uint) -> String {
    let mut string = String::new();
    for i in range(1u, times) { string.push(c) }
    string
}

fn main() {
    for line in std::io::stdin().lines() {
        if line.is_ok() {
            let eval = parse_expr(line.unwrap());
            
            if eval.is_err() {
                println!("error");
            }
        }
    }
}

fn parse_expr(src: String) -> Result<(), ErrorPosition> {
    let mut lexer  = Lexer { reader: StringReader::new(src) };
    let mut parser = try!(Parser::new(&mut lexer));
    let root = parser.parse();

    let table = try!(root.truth_table());
    let mut vars = root.get_variables();

    vars.sort_by(|a, b| a.cmp(b));

    println!("> Truth table:");
    for var in vars.iter() {
        print!("{}    ", var);
    }
    print!("Result\n\n");

    for &(ref vars, ref res) in table.iter() {
        let mut sorted = Vec::with_capacity(table.len());
        for pair in vars.iter() { sorted.push(pair); }
        sorted.sort_by(|a, b| a.val0().cmp(b.val0()));
        for &(ref name, ref val) in sorted.iter() {
            print!("{}{}    ", **val as u8, repeat_char(' ', name.len()));
        }
        print!("{}\n", *res as u8);    
    }
    
    println!("> Parsed tree:\n{}", root);
    println!("> Variables: {}", root.get_variables());
    Ok(())
}
