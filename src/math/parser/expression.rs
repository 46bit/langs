use super::*;
use self::shunting_yard::*;
use math::*;
use nom::is_digit;

named!(pub expressions<&[u8], Vec<Expression>>,
  separated_list!(ws!(tag!(",")), call!(expression)));

named!(pub expression<&[u8], Expression>,
    do_parse!(
        first_operand: ws!(call!(operand)) >>
        shunting_yard: fold_many0!(
            pair!(ws!(call!(operator)), ws!(call!(operand))),
            ShuntingYard::new(first_operand),
            |mut shunting_yard: ShuntingYard, (operator, operand)| {
                shunting_yard.push(operator, operand);
                shunting_yard
            }
        ) >>
        (shunting_yard.into_expression())));

named!(operator<&[u8], Operator>,
  map!(one_of!("+-*/"), |o| match o {
    '+' => Operator::Add,
    '-' => Operator::Subtract,
    '*' => Operator::Multiply,
    '/' => Operator::Divide,
    _ => unreachable!()
  }));

named!(operand<&[u8], Operand>,
  alt_complete!(
    map!(i64, Operand::I64) |
    map!(variable_substitution, Operand::VarSubstitution) |
    map!(function_application, |t| Operand::FnApplication(t.0, t.1))));

named!(i64<&[u8], i64>,
  map!(
    take_while1!(|b: u8| is_digit(b) || b == b'-'),
    |i| to_str(i).unwrap().parse().unwrap()));

named!(variable_substitution<&[u8], Name>,
  do_parse!(
    name: call!(variable_name) >>
    ws!(peek!(none_of!("("))) >>
    (name)));

named!(function_application<&[u8], (Name, Vec<Expression>)>,
  do_parse!(
    name: call!(function_name) >>
    ws!(tag!("(")) >>
    expressions: map!(opt!(call!(expressions)), Option::unwrap_or_default) >>
    ws!(tag!(")")) >>
    (name, expressions)));

#[cfg(test)]
mod tests {
    use super::*;
    use nom::{self, IResult};

    #[test]
    fn expression_test() {
        assert_eq!(
            expression(b"1"),
            as_done(b"", Expression::Operand(Operand::I64(1)))
        );
        assert_eq!(
            expression(b"-123"),
            as_done(b"", Expression::Operand(Operand::I64(-123)))
        );
        assert_eq!(
            expression(b"-123;"),
            as_done(b";", Expression::Operand(Operand::I64(-123)))
        );
        assert_eq!(
            expression(b"35 + -12"),
            as_done(
                b"",
                Expression::Operation(
                    Operator::Add,
                    box Expression::Operand(Operand::I64(35)),
                    box Expression::Operand(Operand::I64(-12))
                )
            )
        );
        assert_eq!(
            expression(b"35 - i;"),
            as_done(
                b";",
                Expression::Operation(
                    Operator::Subtract,
                    box Expression::Operand(Operand::I64(35)),
                    box Expression::Operand(Operand::VarSubstitution(as_name("i")))
                )
            )
        );
        assert_eq!(
            expression(b"foo * bar;"),
            as_done(
                b";",
                Expression::Operation(
                    Operator::Multiply,
                    box Expression::Operand(Operand::VarSubstitution(as_name("foo"))),
                    box Expression::Operand(Operand::VarSubstitution(as_name("bar"))),
                )
            )
        );
        assert_eq!(
            expression(b"f(5) + bar * fn(1, 2, -3, f(i, foo / 3, 9) - -3);"),
            as_done(
                b";",
                Expression::Operation(
                    Operator::Add,
                    box Expression::Operand(Operand::FnApplication(
                        as_name("f"),
                        vec![Expression::Operand(Operand::I64(5))]
                    )),
                    box Expression::Operation(
                        Operator::Multiply,
                        box Expression::Operand(Operand::VarSubstitution(as_name("bar"))),
                        box Expression::Operand(Operand::FnApplication(
                            as_name("fn"),
                            vec![
                                Expression::Operand(Operand::I64(1)),
                                Expression::Operand(Operand::I64(2)),
                                Expression::Operand(Operand::I64(-3)),
                                Expression::Operation(
                                    Operator::Subtract,
                                    box Expression::Operand(Operand::FnApplication(
                                        as_name("f"),
                                        vec![
                                            Expression::Operand(Operand::VarSubstitution(
                                                as_name("i"),
                                            )),
                                            Expression::Operation(
                                                Operator::Divide,
                                                box Expression::Operand(Operand::VarSubstitution(
                                                    as_name("foo"),
                                                )),
                                                box Expression::Operand(Operand::I64(3)),
                                            ),
                                            Expression::Operand(Operand::I64(9)),
                                        ],
                                    )),
                                    box Expression::Operand(Operand::I64(-3)),
                                ),
                            ]
                        ))
                    )
                )
            )
        );
    }

    #[test]
    fn operator_test() {
        assert_eq!(operator(b"+"), as_done(b"", Operator::Add));
        assert_eq!(operator(b"-"), as_done(b"", Operator::Subtract));
        assert_eq!(operator(b"*"), as_done(b"", Operator::Multiply));
        assert_eq!(operator(b"/"), as_done(b"", Operator::Divide));
        assert_eq!(operator(b"^"), IResult::Error(nom::ErrorKind::OneOf));
        assert_eq!(operator(b"+ "), as_done(b" ", Operator::Add));
    }

    #[test]
    fn operand_test() {
        assert_eq!(operand(b"1"), as_done(b"", Operand::I64(1)));
        assert_eq!(operand(b"794"), as_done(b"", Operand::I64(794)));
        assert_eq!(operand(b"-1"), as_done(b"", Operand::I64(-1)));
        assert_eq!(operand(b"-390"), as_done(b"", Operand::I64(-390)));

        assert_eq!(
            operand(b"f)"),
            as_done(b")", Operand::VarSubstitution(as_name("f")))
        );
        assert_eq!(
            operand(b"foo * 5"),
            as_done(b"* 5", Operand::VarSubstitution(as_name("foo")))
        );

        assert_eq!(
            operand(b"fn(k * 5)"),
            as_done(
                b"",
                Operand::FnApplication(
                    as_name("fn"),
                    vec![
                        Expression::Operation(
                            Operator::Multiply,
                            box Expression::Operand(Operand::VarSubstitution(as_name("k"))),
                            box Expression::Operand(Operand::I64(5)),
                        ),
                    ]
                )
            )
        );
        assert_eq!(
            operand(b"j(3 + foo, l + 3 - 2)"),
            as_done(
                b"",
                Operand::FnApplication(
                    as_name("j"),
                    vec![
                        Expression::Operation(
                            Operator::Add,
                            box Expression::Operand(Operand::I64(3)),
                            box Expression::Operand(Operand::VarSubstitution(as_name("foo"))),
                        ),
                        Expression::Operation(
                            Operator::Subtract,
                            box Expression::Operation(
                                Operator::Add,
                                box Expression::Operand(Operand::VarSubstitution(as_name("l"))),
                                box Expression::Operand(Operand::I64(3)),
                            ),
                            box Expression::Operand(Operand::I64(2)),
                        ),
                    ]
                )
            )
        );
    }

    #[test]
    fn i64_test() {
        assert_eq!(i64(b"1"), as_done(b"", 1));
        assert_eq!(i64(b"794"), as_done(b"", 794));
        assert_eq!(i64(b"-1"), as_done(b"", -1));
        assert_eq!(i64(b"-390"), as_done(b"", -390));
        assert_eq!(i64(b"a"), IResult::Error(nom::ErrorKind::TakeWhile1));
    }

    #[test]
    fn variable_substitution_test() {
        assert_eq!(
            variable_substitution(b"i"),
            IResult::Incomplete(nom::Needed::Size(2))
        );
        assert_eq!(variable_substitution(b"i +"), as_done(b"+", as_name("i")));
        assert_eq!(
            variable_substitution(b"foo * 5"),
            as_done(b"* 5", as_name("foo"))
        );
        assert_eq!(
            variable_substitution(b"fn("),
            IResult::Error(nom::ErrorKind::NoneOf)
        );
    }

    #[test]
    fn function_application_test() {
        assert_eq!(
            function_application(b"f"),
            IResult::Incomplete(nom::Needed::Size(2))
        );
        assert_eq!(
            function_application(b"f()"),
            as_done(b"", (as_name("f"), vec![]))
        );
        assert_eq!(
            function_application(b"f(5)"),
            as_done(
                b"",
                (as_name("f"), vec![Expression::Operand(Operand::I64(5))])
            )
        );
        assert_eq!(
            function_application(b"f(5, 6)"),
            as_done(
                b"",
                (
                    as_name("f"),
                    vec![
                        Expression::Operand(Operand::I64(5)),
                        Expression::Operand(Operand::I64(6)),
                    ]
                )
            )
        );
        assert_eq!(
            function_application(b"f(a)"),
            as_done(
                b"",
                (
                    as_name("f"),
                    vec![Expression::Operand(Operand::VarSubstitution(as_name("a")))]
                )
            )
        );
        assert_eq!(
            function_application(b"fn(i, j)"),
            as_done(
                b"",
                (
                    as_name("fn"),
                    vec![
                        Expression::Operand(Operand::VarSubstitution(as_name("i"))),
                        Expression::Operand(Operand::VarSubstitution(as_name("j"))),
                    ]
                )
            )
        );
        assert_eq!(
            function_application(b"fn(k * 5)"),
            as_done(
                b"",
                (
                    as_name("fn"),
                    vec![
                        Expression::Operation(
                            Operator::Multiply,
                            box Expression::Operand(Operand::VarSubstitution(as_name("k"))),
                            box Expression::Operand(Operand::I64(5)),
                        ),
                    ]
                )
            )
        );
        assert_eq!(
            function_application(b"fn(3 + foo, l + 3 - 2)"),
            as_done(
                b"",
                (
                    as_name("fn"),
                    vec![
                        Expression::Operation(
                            Operator::Add,
                            box Expression::Operand(Operand::I64(3)),
                            box Expression::Operand(Operand::VarSubstitution(as_name("foo"))),
                        ),
                        Expression::Operation(
                            Operator::Subtract,
                            box Expression::Operation(
                                Operator::Add,
                                box Expression::Operand(Operand::VarSubstitution(as_name("l"))),
                                box Expression::Operand(Operand::I64(3)),
                            ),
                            box Expression::Operand(Operand::I64(2)),
                        ),
                    ]
                )
            )
        );
        assert_eq!(
            function_application(b"f +"),
            IResult::Error(nom::ErrorKind::Tag)
        );
    }

    fn as_name(s: &str) -> Name {
        Name(s.to_string())
    }

    fn as_done<O, E>(remaining: &[u8], output: O) -> IResult<&[u8], O, E> {
        IResult::Done(&remaining[..], output)
    }
}
