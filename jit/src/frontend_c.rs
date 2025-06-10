use crate::transpiler::{MyExpression, MyLabel, MyStatement, StatementObject, TypeMap, TypeResolver, Varindex};
use thiserror::__private::Var;

pub struct FrontendC {
    type_map: TypeMap,
    statement_object: StatementObject,
    output: String,
}

impl FrontendC {
    pub fn new(type_map: TypeMap, statement_object: StatementObject) -> Self {
        FrontendC {
            type_map,
            statement_object,
            output: String::new(),
        }
    }

    pub fn transpile(&mut self) {
        self.output.push_str(format!("int hello() {{\n").as_str());
        for (my_label, statement_block) in self.statement_object.blocks.clone().iter() {
            match my_label {
                MyLabel::Entry => {
                    self.output.push_str("label_entry:\n");
                }
                MyLabel::PyLabel(idx) => {
                    self.output.push_str(format!("label_{}:\n", idx).as_str());
                }
            }
            for statement in statement_block.statements.clone().iter() {
                match statement {
                    MyStatement::Assign(lhs, rhs) => {
                        self.handle_assignment(lhs, rhs);
                    }
                    MyStatement::Jump(_) => {}
                    MyStatement::JumpIfTrue(_, _) => {}
                    MyStatement::Return(_) => {}
                    MyStatement::PlaceholderJump(_, _) => {}
                    MyStatement::PlaceholderTarget(_) => {}
                    MyStatement::Noop => {}
                }
            }
        }

        println!("{}", self.output);
    }

    fn handle_assignment(&mut self, lhs: &Varindex, rhs: &MyExpression) {
        match rhs {
            MyExpression::Varname(_) => {}
            MyExpression::Variable(rhs) => {
                self.output.push_str(format!("int var_{} = var_{};\n", lhs.0, rhs.0).as_str());
            }
            MyExpression::Int(_) => {}
            MyExpression::Float64(_) => {}
            MyExpression::String(_) => {}
            MyExpression::Struct(_) => {}
            MyExpression::UnaryOp(_, _) => {}
            MyExpression::BinaryOp(_, _, _) => {}
            MyExpression::CompareOp(_, _, _) => {}
            MyExpression::Subscript(_, _) => {}
            MyExpression::GlobalFunction(_) => {}
            MyExpression::Call(_, _) => {}
            MyExpression::ConvertToIterator(_) => {}
            MyExpression::IteratorCheck(_) => {}
            MyExpression::IteratorNext(_) => {}
        }
    }
}
