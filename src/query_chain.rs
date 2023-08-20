use crate::chain::Blockchain;
use crate::pratt;
use crate::record::Record;
use std::collections::HashMap;
use std::str::FromStr;
/** Let us say that we impose a structure where a given queryable record has a list of keys and values **/

pub enum QueryableError {}

pub trait Queryable<F>
where
    F: Fn(HashMap<String, String>) -> Result<bool, String>,
{
    fn query(&self, predicate: F) -> Result<Vec<Record>, String>;
}

impl<F> Queryable<F> for Blockchain
where
    F: Fn(HashMap<String, String>) -> Result<bool, String>,
{
    fn query(&self, predicate: F) -> Result<Vec<Record>, String> {
        let mut records: Vec<Record> = vec![];
        for block in self {
            for b in block.data.iter() {
                match b.structured_entry() {
                    Ok(r) => match predicate(r.pairs()) {
                        Ok(out) => {
                            if out {
                                records.push(b.clone());
                            }
                        }
                        Err(_) => {}
                    },
                    Err(_) => {}
                }
            }
        }
        Ok(records)
    }
}

// The Query struct and implementation handles a side-effect free query language
pub struct Query {
    input: String,
}

impl Query {
    // New query or failure. Does not eval query; simply lexes/parses.
    pub fn new(input: String) -> Result<Query, String> {
        pratt::lex_parse(input.clone()).map_err(|e| e.to_string())?;
        Ok(Query { input })
    }

    // The parse function reads the input and returns a closure that will take a HashMap, execute the
    // logic in the input, and return true or false.
    pub fn parse(&self) -> impl Fn(HashMap<String, String>) -> Result<bool, String> {
        let boxed_text = Box::new(self.input.clone());
        move |env: HashMap<String, String>| -> Result<bool, String> {
            let text = boxed_text.clone();
            let mut parser_env = HashMap::<String, pratt::Value>::new();

            for (k, v) in env {
                match i32::from_str(&v).map(|i| parser_env.insert(k.clone(), pratt::Value::Num(i)))
                {
                    Ok(_) => break,
                    Err(_) => {}
                }
                match bool::from_str(&v)
                    .map(|b| parser_env.insert(k.clone(), pratt::Value::Bool(b)))
                {
                    Ok(_) => break,
                    Err(_) => {}
                }
                parser_env.insert(k.clone(), pratt::Value::Str(v));
            }

            let ast = pratt::lex_parse(text.to_string()).map_err(|e| e.to_string())?;
            let result = pratt::eval(ast, &parser_env).map_err(|e| e.to_string())?;
            match result {
                pratt::Value::Bool(b) => Ok(b),
                _ => Err("result is not a boolean".to_string()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Query;
    use std::collections::HashMap;

    #[test]
    fn test_query() {
        let q = Query::new("x == \"bar\"".to_string()).unwrap();
        let env: HashMap<String, String> = HashMap::from([("x".to_string(), "bar".to_string())]);
        let f = q.parse();
        assert_eq!(f(env), Ok(true));
    }
}
