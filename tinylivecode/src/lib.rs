#![no_std]

use micromath::F32Ext;

const MAX_STACK_SIZE: usize = 64;

pub enum ParseError {
    TooManyValuesInStack,
    NoValuesInStack,
    NumberNotParsed,
}

struct Stack<T> {
    stack: [Option<T>; MAX_STACK_SIZE],
    stack_top: usize,
}
impl<T: Copy> Stack<T> {
    fn new() -> Stack<T> {
        Stack {
            stack: [Option::None; MAX_STACK_SIZE],
            stack_top: 0,
        }
    }

    fn push(&mut self, val: T) -> Result<(), ParseError> {
        if self.stack_top >= MAX_STACK_SIZE {
            return Err(ParseError::TooManyValuesInStack);
        }

        self.stack[self.stack_top] = Some(val);
        self.stack_top += 1;
        Ok(())
    }

    fn pop(&mut self) -> Option<T> {
        if self.stack_top == 0 {
            return None;
        }

        // this will never be empty, i promise
        self.stack_top -= 1;
        let result = self.stack[self.stack_top].take();

        result
    }

    fn iter<'a>(&'a self) -> StackIter<'a, T> {
        StackIter {
            curr_loc: 0,
            end_loc: self.stack_top,
            data: self,
        }
    }
}

struct StackIter<'a, T> {
    curr_loc: usize,
    end_loc: usize,
    data: &'a Stack<T>,
}

impl<'a, T> StackIter<'a, T> {
    fn is_empty(&self) -> bool {
        self.curr_loc == self.end_loc
    }
}

impl<'a, T: Copy> Iterator for StackIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_empty() {
            None
        } else {
            let result = self.data.stack[self.curr_loc]; // there is a case that _shouldn't_ happen where this is None
            self.curr_loc += 1;
            result
        }
    }
}

pub struct TinyExpr {
    stack: Stack<ExprNode>,
}

impl TinyExpr {
    pub fn from_str(s: &str) -> ParseResult<TinyExpr> {
        let tokens = s.split_whitespace();

        let mut stack = Stack::new();

        for token in tokens {
            let val = ExprNode::parse_token(token)?;
            stack.push(val)?;
        }

        Ok(Self { stack })
    }

    pub fn eval(&self, x: f32, y: f32, t: f32) -> ParseResult<f32> {
        let mut eval_stack = Stack::new();

        let mut stack_iterator = self.stack.iter();

        while let Some(curr_val) = stack_iterator.next() {
            // println!("curr_val {:?}", curr_val);
            // println!("eval_stack {:?}", eval_stack.stack);
            match curr_val {
                ExprNode::Lerp => {
                    let z: f32 = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    let y: f32 = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    let x = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    eval_stack.push((1.0 - z) * x + z * y)?;
                }
                // binary
                ExprNode::Add => {
                    let y: f32 = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    let x = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;

                    eval_stack.push(x + y)?;
                }
                ExprNode::Mul => {
                    let y: f32 = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    let x = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    eval_stack.push(x * y)?;
                }
                ExprNode::Div => {
                    let y: f32 = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    let x = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    eval_stack.push(x / y)?;
                }
                ExprNode::Sub => {
                    let y: f32 = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    let x = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    eval_stack.push(x - y)?;
                }
                ExprNode::Pow => {
                    let y: f32 = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    let x = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    eval_stack.push(x.powf(y))?;
                }
                ExprNode::Atan2 => {
                    let y: f32 = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    let x: f32 = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    eval_stack.push(x.atan2(y))?;
                }

                // unary
                ExprNode::Sin => {
                    let x = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    eval_stack.push(x.sin())?;
                }
                ExprNode::Cos => {
                    let x = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    eval_stack.push(x.cos())?;
                }
                ExprNode::Fract => {
                    let x = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    eval_stack.push(x.fract())?;
                }
                ExprNode::Sqrt => {
                    let x = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    eval_stack.push(x.fract())?;
                }
                ExprNode::Floor => {
                    let x = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    eval_stack.push(x.floor())?;
                }
                ExprNode::Abs => {
                    let x = eval_stack
                        .pop()
                        .ok_or_else(|| ParseError::NoValuesInStack)?;
                    eval_stack.push(x.abs())?;
                }
                // no variable
                ExprNode::Number(r) => eval_stack.push(r)?,
                ExprNode::X => eval_stack.push(x)?,
                ExprNode::Y => eval_stack.push(y)?,
                ExprNode::T => eval_stack.push(t)?,
            };
        }

        if let Some(x) = eval_stack.pop() {
            Ok(x)
        } else {
            Err(ParseError::NoValuesInStack)
        }
    }
}

pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Copy, Clone, Debug)]
enum ExprNode {
    Number(f32),
    X,
    Y,
    T,
    // unary
    Sin,
    Cos,
    Fract,
    Sqrt,
    Floor,
    Abs,
    // binary
    Atan2,
    Pow,
    Add,
    Mul,
    Div,
    Sub,
    // ternary
    Lerp,
}

impl ExprNode {
    fn parse_token(token: &str) -> ParseResult<ExprNode> {
        match token {
            "x" => Ok(ExprNode::X),
            "y" => Ok(ExprNode::Y),
            "t" => Ok(ExprNode::T),
            "sin" => Ok(ExprNode::Sin),
            "cos" => Ok(ExprNode::Cos),
            "atan2" => Ok(ExprNode::Atan2),
            "fract" => Ok(ExprNode::Fract),
            "pow" => Ok(ExprNode::Pow),
            "sqrt" => Ok(ExprNode::Sqrt),
            "floor" => Ok(ExprNode::Floor),
            "abs" => Ok(ExprNode::Abs),
            "add" => Ok(ExprNode::Add),
            "mul" => Ok(ExprNode::Mul),
            "div" => Ok(ExprNode::Div),
            "sub" => Ok(ExprNode::Sub),
            "lerp" => Ok(ExprNode::Lerp),
            _ => {
                let float = token
                    .parse::<f32>()
                    .map_err(|_| ParseError::NumberNotParsed)?;
                Ok(ExprNode::Number(float))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    enum TestEvalResult {
        Ok(f32),
        ErrFromStr(ParseError),
        ErrEval(ParseError),
    }

    impl TestEvalResult {
        /// Returns `true` if the test eval result is [`Ok`].
        ///
        /// [`Ok`]: TestEvalResult::Ok
        #[must_use]
        fn is_ok(&self) -> bool {
            matches!(self, Self::Ok(..))
        }

        /// Returns `true` if the test eval result is [`ErrFromStr`].
        ///
        /// [`ErrFromStr`]: TestEvalResult::ErrFromStr
        #[must_use]
        fn is_err_from_str(&self) -> bool {
            matches!(self, Self::ErrFromStr(..))
        }

        /// Returns `true` if the test eval result is [`ErrEval`].
        ///
        /// [`ErrEval`]: TestEvalResult::ErrEval
        #[must_use]
        fn is_err_eval(&self) -> bool {
            matches!(self, Self::ErrEval(..))
        }
    }

    fn evaluate(s: &str, x: f32, y: f32, t: f32) -> TestEvalResult {
        let e = match TinyExpr::from_str(s) {
            Ok(r) => match r.eval(x, y, t) {
                Ok(a) => TestEvalResult::Ok(a),
                Err(err) => TestEvalResult::ErrEval(err),
            },
            Err(err) => TestEvalResult::ErrFromStr(err),
        };

        // can be useful to debug if you introduce std again
        // match &e {
        //     TestEvalResult::Ok(e) => println!("Ok, {}", e),
        //     TestEvalResult::ErrFromStr(parse_error) => println!("{}", "err from str"),
        //     TestEvalResult::ErrEval(parse_error) => println!("{}", "err from eval"),
        // };

        e
    }

    #[test]
    fn it_works() {
        match evaluate("2.3", 0.0, 0.0, 1.0) {
            TestEvalResult::Ok(r) => assert_eq!(r, 2.3),
            _ => assert!(false),
        }

        match evaluate("2", 0.0, 0.0, 1.0) {
            TestEvalResult::Ok(r) => assert_eq!(r, 2.0),
            _ => assert!(false),
        }

        match evaluate("x", 5.0, 0.0, 1.0) {
            TestEvalResult::Ok(r) => assert_eq!(r, 5.0),
            _ => assert!(false),
        }

        match evaluate("y", 0.0, 6.0, 1.0) {
            TestEvalResult::Ok(r) => assert_eq!(r, 6.0),
            _ => assert!(false),
        }

        match evaluate("x y add", 2.0, 5.0, 1.0) {
            TestEvalResult::Ok(r) => {
                assert_eq!(r, 7.0)
            }
            _ => {
                assert!(false)
            }
        }

        match evaluate("x y 0.0 lerp", 2.0, 5.0, 1.0) {
            TestEvalResult::Ok(r) => {
                assert_eq!(r, 2.0)
            }
            _ => {
                assert!(false)
            }
        }

        match evaluate("x y 1.0 lerp", 2.0, 5.0, 1.0) {
            TestEvalResult::Ok(r) => {
                assert_eq!(r, 5.0)
            }
            _ => {
                assert!(false)
            }
        }

        match evaluate("x y mul 4.0 add", 2.0, 5.0, 1.0) {
            TestEvalResult::Ok(r) => {
                assert_eq!(r, 14.0)
            }
            _ => {
                assert!(false)
            }
        }
    }

    #[test]
    fn it_does_not_work_empty() {
        let empty_result = evaluate("", 0.0, 0.0, 1.0);
        assert!(empty_result.is_err_eval());
    }

    #[test]
    fn it_does_not_work_gobbligook() {
        let empty_result = evaluate("gobbligook", 0.0, 0.0, 1.0);
        assert!(empty_result.is_err_from_str());
    }
}
