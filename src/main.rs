use nom::character::complete::*;
use nom::bytes::complete::*;
use nom::sequence::delimited;
use nom::Parser;
use nom::{IResult, error::Error};
use nom::branch::alt;
use nom::sequence::preceded;

#[derive(Debug)]
struct Comando{
  digitado: String,
  colunas: String,
  arquivo: String,
  predicados: bool
}

fn novo_comando(input: String) -> Comando{

  //essa linha tenta pegar oque não for " " depois de "Select ", retornando Ok((resto, resultado)) ou Err(algo)
  let resultado: IResult<&str, &str, Error<&str>> = preceded(tag_no_case("SELECT "), is_not(" ")).parse(&input);
  //essa extrai uma tupla do resultado, mandando mensagem caso seja Err
  let parse_select = resultado.expect("erro no parse do select");
  //pega o resultado, nesse caso as colunas
  let colunas = String::from(parse_select.1);

  //essa linha tenta pegar oque não for " " depois de " FROM " do resto do parsing anteiror, retornando Ok((resto, resultado)) ou Err(algo)
  let resultado: IResult<&str, &str, Error<&str>> = preceded(tag_no_case(" FROM "), is_not(" ")).parse(parse_select.0);
  //essa extrai uma tupla do resultado, mandando mensagem caso seja Err
  let parse_from = resultado.expect("erro no parse do from");
  //pega o resultado, nesse caso o arquivo
  let arquivo = String::from(parse_from.1);

  //falta fazer o parse do WHERE
  
  Comando{
    digitado: input.clone(),
    colunas: colunas,
    arquivo: arquivo,
    predicados: true
  }
}


fn main() {
  //teste
  println!("{:?}", novo_comando(String::from("Select coluna from arquivo where")));
}
