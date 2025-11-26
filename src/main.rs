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
  predicados: String,
}

fn novo_comando(input: String) -> Comando{

  //essa linha tenta pegar oque não for " " depois de "Select ", retornando Ok((resto, resultado)) ou Err(algo)
  let resultado: IResult<&str, &str, Error<&str>> = preceded(tag_no_case("SELECT "), is_not(" ")).parse(&input);
  //essa extrai uma tupla do resultado, mandando mensagem caso seja Err
  let parse_select = resultado.expect("\n///ERRO NO PARSE DO SELECT ====>");
  //pega o resultado, nesse caso as colunas
  let colunas = String::from(parse_select.1);

  //essa linha tenta pegar oque não for " " depois de " FROM " do resto do parsing anteiror, retornando Ok((resto, resultado)) ou Err(algo)
  let resultado: IResult<&str, &str, Error<&str>> = preceded(tag_no_case(" FROM "), is_not(" ")).parse(parse_select.0);
  //essa extrai uma tupla do resultado, mandando mensagem caso seja Err
  let parse_from = resultado.expect("\n///ERRO NO PARSE DO FROM ====>");
  //pega o resultado, nesse caso o arquivo
  let arquivo = String::from(parse_from.1);

  //essa linha tenta pegar oque não for ";" depois de " WHERE " do resto do parsing anteiror, retornando Ok((resto, resultado)) ou Err(algo)
  let resultado: IResult<&str, &str, Error<&str>> = preceded(tag_no_case(" WHERE "), is_not(";")).parse(parse_from.0);
  //essa extrai uma tupla do resultado, mandando mensagem caso seja Err
  let parse_where = resultado.expect("\n///ERRO NO PARSE DO WHERE ====>");
  //pega o resultado, nesse caso o arquivo
  let predicados = String::from(parse_where.1);
  
  Comando{
    digitado: input.clone(),
    colunas: colunas,
    arquivo: arquivo,
    predicados: predicados
  }
}


fn main() {
  //teste
  println!("{:?}", novo_comando(String::from("Selectcoluna from arquivo where alunos>2 AND media>=6;")));
}
