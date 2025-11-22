use nom::character::complete::*;
use nom::bytes::complete::*;
use nom::sequence::delimited;
use nom::Parser;
use nom::{IResult, error::Error};

struct Comando{
  digitado: String,
  colunas: &str,
  arquivo: &str,
  predicados: todo!()
}

struct SELECT<'a>{
  colunas: &'a str
}

struct FROM<'a>{
  arquivo: &'a str
}

struct WHERE{
  predicados: bool
  //tô em duvida de como fazer esse
  //seria o resultado dos predicados? codei como se fosse
}

impl Default for WHERE {
    fn default() -> Self {
        WHERE {
            predicados: true
        }
    }
}

fn parse_select(input: &str) -> SELECT{
  
  //associei o resultado do expect para outra variavel
  //fiz isso para que o tag_no_case pudesse inferir o tipo de Error, uma solução rapida
  let resultado: IResult<&str, &str, Error<&str>> = delimited(tag_no_case("SELECT "), is_not(" "), char(' ')).parse(input);
  let outro = resultado.expect("erro");
  let selecionar = SELECT{
    colunas: outro.1
  };
  selecionar

}

fn parse_from(input: &str) -> FROM{
  
  let resultado: IResult<&str, &str, Error<&str>> = delimited(tag_no_case("FROM "), is_not(" "), char(' ')).parse(input);
  let outro = resultado.expect("erro");
  let de = FROM{
    arquivo: outro.1
  };
  de

}

fn parse_where(input: &str) -> WHERE{
  
  //seria aqui iriamos separar os predicados e ver o resultado?
  todo!();
}

fn main() {
  println!("Hello, world!");
}
