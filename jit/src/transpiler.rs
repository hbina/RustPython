use crate::JitType;
use num_traits::ToPrimitive;
use rustpython_compiler_core::bytecode::{BinaryOperator, BorrowedConstant, CodeObject, CodeUnit, ComparisonOperator, Constant, Instruction, OpArg, OpArgState, UnaryOperator};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Varindex(pub usize);

impl Varindex {
    pub fn as_string(&self) -> String {
        format!("varidx.{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub enum MyExpression {
    Varname(String),
    Variable(Varindex),
    // None,
    Int(i64),
    Float64(f64),
    String(String),
    Struct(Vec<Varindex>),
    UnaryOp(UnaryOperator, Varindex),
    BinaryOp(BinaryOperator, Varindex, Varindex),
    CompareOp(ComparisonOperator, Varindex, Varindex),
    Subscript(Varindex, Varindex),
    GlobalFunction(String),
    Call(Varindex, Vec<Varindex>),
    ConvertToIterator(Varindex),
    IteratorCheck(Varindex),
    IteratorNext(Varindex),
}

impl MyExpression {
    pub fn as_string(&self) -> String {
        match self {
            MyExpression::Varname(s) => format!("$varname {}", s),
            MyExpression::Variable(v) => format!("$var {}", v.as_string()),
            // MyExpression::None => "$none".to_string(),
            MyExpression::Int(v) => format!("$int {}", v),
            MyExpression::Float64(v) => format!("$float {}", v),
            MyExpression::String(v) => format!("$string {}", v),
            MyExpression::Struct(v) => {
                let s = v.iter().map(|x| x.as_string()).collect::<Vec<String>>().join(", ");
                format!("$struct [{}]", s)
            }
            MyExpression::UnaryOp(op, a) => format!("$unary_op {:?}({})", op, a.as_string()),
            MyExpression::BinaryOp(op, a, b) => format!("$binary_op {:?}({}, {})", op, a.as_string(), b.as_string()),
            MyExpression::CompareOp(op, a, b) => format!("$compare_op {:?}({}, {})", op, a.as_string(), b.as_string()),
            MyExpression::Subscript(a, b) => format!("$subscript {}[{}]", a.as_string(), b.as_string()),
            MyExpression::GlobalFunction(name) => format!("$global {}", name),
            MyExpression::Call(function, args) => {
                format!("$call {} ({})", function.as_string(), args.iter().map(|v| v.as_string()).collect::<Vec<String>>().join(", "))
            }
            MyExpression::ConvertToIterator(varindex) => format!("$iterator_convert {}", varindex.as_string()),
            MyExpression::IteratorCheck(varindex) => format!("$iterator_check {}", varindex.as_string()),
            MyExpression::IteratorNext(varindex) => format!("$iterator_next {}", varindex.as_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MyStatement {
    Assign(Varindex, MyExpression),
    Jump(usize),
    JumpIfTrue(Varindex, usize),
    Return(Varindex),
    PlaceholderJump(Varindex, usize),
    PlaceholderTarget(usize),
    Noop,
}

impl Display for MyStatement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MyStatement::Assign(lhs, rhs) => {
                write!(f, "{} = {}", lhs.as_string(), rhs.as_string())
            }
            MyStatement::Jump(target) => write!(f, "jump {}", target),
            MyStatement::JumpIfTrue(lhs, target) => write!(f, "jump {} if {}", target, lhs.as_string()),
            MyStatement::Return(lhs) => {
                write!(f, "return {}", lhs.as_string())
            }
            MyStatement::PlaceholderJump(lhs, id) => {
                write!(f, "placeholder_jump {} if {}", id, lhs.as_string())
            }
            MyStatement::PlaceholderTarget(id) => {
                write!(f, "placeholder_target {}", id)
            }
            MyStatement::Noop => {
                write!(f, "noop")
            }
        }
    }
}

impl MyStatement {}

#[derive(Clone, Default)]
struct MyBlock {
    // pub statements: Vec<MyStatement>,
    // comments: HashMap<VarIndex, String>,
}

#[derive(Clone)]
pub struct FunctionIr {
    pub blocks: BTreeMap<usize, MyBlock>,
    pub varname_to_varindex: HashMap<String, Varindex>,
}

#[derive(Debug, Clone)]
pub struct StatementBlock {
    // pub label: Option<usize>,
    // pub block_id: usize,
    pub source: Option<CodeUnit>,
    pub statements: Vec<MyStatement>,
}

#[derive(Debug, Clone)]
pub struct StatementObject {
    pub varname_to_varindex: HashMap<String, Varindex>,
    pub varnum_to_varindex: HashMap<usize, Varindex>,
    pub constindex_to_varindex: HashMap<usize, Varindex>,
    pub blocks: BTreeMap<MyLabel, StatementBlock>,
    pub varindex_counter: usize,
}

impl StatementObject {
    pub fn varindex_counter(&self) -> usize {
        self.varindex_counter
    }

    pub fn print_transformation(&self) {
        for (label, statement_block) in self.blocks.iter() {
            match statement_block.source {
                Some(source) => {
                    println!("label: {} {}", label, stringify_code_object(source));
                }
                None => {
                    println!("label: {}", label);
                }
            };
            for statement in statement_block.statements.iter() {
                println!("\t{}", statement);
            }
        }
    }

    pub fn print_flatten(&self) {
        let mut global_idx = 0;
        println!("label\tglobal\tlocal\tstatement");
        for (label, statement_block) in self.blocks.iter() {
            for (statement_idx, statement) in statement_block.statements.iter().enumerate() {
                let label_str = match label {
                    MyLabel::Entry => "-".to_string(),
                    MyLabel::PyLabel(label) => label.to_string(),
                };

                println!("{}\t{}\t{}\t{}", label_str, global_idx, statement_idx, statement);
                global_idx += 1;
            }
        }
    }
}

fn collect_blocks<C>(code_object: &CodeObject<C>) -> HashMap<usize, StatementBlock>
where
    C: Constant,
{
    let mut arg_state = OpArgState::default();
    let mut result = HashMap::new();
    for (label, code_unit) in code_object.instructions.iter().enumerate() {
        let (instruction, arg) = arg_state.get(*code_unit);
        match instruction {
            Instruction::ExtendedArg => todo!(),
            Instruction::JumpIfFalse { target } => {
                let target = target.get(arg);
                result.insert(
                    target.0 as usize,
                    StatementBlock {
                        source: Some(*code_unit),
                        statements: Vec::new(),
                    },
                );
            }
            Instruction::JumpIfTrue { target } => {
                let target = target.get(arg);
                result.insert(
                    target.0 as usize,
                    StatementBlock {
                        source: Some(*code_unit),
                        statements: Vec::new(),
                    },
                );
            }
            Instruction::Jump { target } => {
                let target = target.get(arg);
                result.insert(
                    target.0 as usize,
                    StatementBlock {
                        source: Some(*code_unit),
                        statements: Vec::new(),
                    },
                );
            }
            _ => {}
        };
    }
    result
}

fn collect_jump_tables<C>(code_object: &CodeObject<C>) -> Vec<usize>
where
    C: Constant,
{
    let mut arg_state = OpArgState::default();
    let mut result = Vec::new();
    for (label, code_unit) in code_object.instructions.iter().enumerate() {
        let (instruction, arg) = arg_state.get(*code_unit);
        match instruction {
            Instruction::ExtendedArg => todo!(),
            Instruction::JumpIfFalse { target } => {
                let target = target.get(arg);
                result.push(target.0 as usize);
            }
            Instruction::JumpIfTrue { target } => {
                let target = target.get(arg);
                result.push(target.0 as usize);
            }
            Instruction::Jump { target } => {
                let target = target.get(arg);
                result.push(target.0 as usize);
            }
            _ => {}
        };
    }
    result
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum MyLabel {
    Entry,
    PyLabel(usize),
}

impl Display for MyLabel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MyLabel::Entry => write!(f, "entry"),
            MyLabel::PyLabel(label) => write!(f, "py_label_{}", label),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TranspileByteCodeToStatementObject<'a, C>
where
    C: Constant,
{
    varname_to_varindex: HashMap<String, Varindex>,
    varnum_to_varindex: HashMap<usize, Varindex>,
    constindex_to_varindex: HashMap<usize, Varindex>,
    globalindex_to_varindex: HashMap<usize, Varindex>,
    blocks: BTreeMap<MyLabel, StatementBlock>,
    varindex_counter: usize,
    jump_placeholder_counter: usize,
    code_object: &'a CodeObject<C>,
    stack_varindex: Vec<Varindex>,
    stack_jump_placeholder: Vec<usize>,
    depth_counter: usize,
    // stack_block: Vec<usize>,
}

impl<'a, C> TranspileByteCodeToStatementObject<'a, C>
where
    C: Constant,
{
    pub fn new(code_object: &'a CodeObject<C>) -> Self {
        println!("=========================");
        // println!("instructions:");
        // for (offset, inst) in code_object.instructions.iter().enumerate() {
        //     println!("{} => {:?} => {:?}", offset, inst.op, inst.arg);
        // }
        println!("names:");
        for (idx, name) in code_object.names.iter().enumerate() {
            println!("{} {}", idx, name.as_ref().to_string());
        }
        println!("varnames:");
        for (idx, varname) in code_object.varnames.iter().enumerate() {
            println!("{} {}", idx, varname.as_ref().to_string());
        }
        println!("constants:");
        for (idx, constant) in code_object.constants.iter().enumerate() {
            println!("{} {:?}", idx, constant.borrow_constant());
        }
        println!("=========================");

        let mut statements = Vec::new();
        let mut varindex_counter = 0;
        let mut varname_to_varindex = HashMap::new();
        let mut varnum_to_varindex = HashMap::new();
        let mut constindex_to_varindex = HashMap::new();
        let mut globalindex_to_varindex = HashMap::new();
        let mut blocks = BTreeMap::new();

        for (globalindex, constant) in code_object.names.iter().enumerate() {
            let constant_str = constant.as_ref().to_string();
            globalindex_to_varindex.insert(globalindex, Varindex(varindex_counter));
            statements.push(MyStatement::Assign(Varindex(varindex_counter), MyExpression::GlobalFunction(constant_str)));
            varindex_counter += 1;
        }

        for (constindex, constant) in code_object.constants.iter().enumerate() {
            constindex_to_varindex.insert(constindex, Varindex(varindex_counter));
            let my_expression = TranspileByteCodeToStatementObject::convert_borrowed_constant_to_my_expression(constant.borrow_constant());
            statements.push(MyStatement::Assign(Varindex(varindex_counter), my_expression));
            varindex_counter += 1;
        }

        for (varnum, varname) in code_object.varnames.iter().enumerate() {
            let varname_str = varname.as_ref().to_string();
            varname_to_varindex.insert(varname_str, Varindex(varindex_counter));
            varnum_to_varindex.insert(varnum, Varindex(varindex_counter));
            varindex_counter += 1;
        }

        blocks.insert(MyLabel::Entry, StatementBlock { source: None, statements });

        TranspileByteCodeToStatementObject {
            varname_to_varindex,
            varnum_to_varindex,
            constindex_to_varindex,
            globalindex_to_varindex,
            blocks,
            varindex_counter,
            jump_placeholder_counter: 0,
            depth_counter: 0,
            code_object,
            stack_varindex: Vec::new(),
            stack_jump_placeholder: Vec::new(),
        }
    }

    fn convert_borrowed_constant_to_my_expression(value: BorrowedConstant<'_, C>) -> MyExpression {
        match value {
            BorrowedConstant::Integer { value } => MyExpression::Int(value.to_i64().unwrap()),
            BorrowedConstant::Float { value } => MyExpression::Float64(value),
            BorrowedConstant::Boolean { value } => {
                if value {
                    MyExpression::Int(1)
                } else {
                    MyExpression::Int(0)
                }
            }
            BorrowedConstant::Str { value } => MyExpression::String(value.as_str().unwrap().to_string()),
            BorrowedConstant::Bytes { .. } => {
                todo!()
            }
            BorrowedConstant::Code { .. } => {
                todo!()
            }
            BorrowedConstant::Tuple { .. } => {
                todo!()
            }
            BorrowedConstant::None => MyExpression::Int(0),
            BorrowedConstant::Complex { .. } => {
                todo!()
            }
            BorrowedConstant::Ellipsis => {
                todo!()
            }
        }
    }

    fn pop_varindex(&mut self) -> Varindex {
        // assert!(offset > 0);
        // VarIndex(self.variable_idx - offset)
        self.stack_varindex.pop().unwrap()
    }

    fn pop_jump_placeholder(&mut self) -> usize {
        // assert!(offset > 0);
        // VarIndex(self.variable_idx - offset)
        self.stack_jump_placeholder.pop().unwrap()
    }

    // fn get_block_stack(&self) -> usize {
    //     *self.stack_block.last().unwrap()
    // }
    //
    // fn push_block_stack(&mut self) {
    //     self.block_id += 1;
    //     self.stack_block.push(self.block_id)
    // }
    //
    // fn pop_block_stack(&mut self) {
    //     self.stack_block.push(self.block_id)
    // }

    pub fn transpile(mut self) -> StatementObject {
        let mut arg_state = OpArgState::default();

        let jump_tables = collect_jump_tables(self.code_object);

        for (label, &code_unit) in self.code_object.instructions.iter().enumerate() {
            let (instruction, arg) = arg_state.get(code_unit);

            // let code_unit_str = stringify_code_object(code_unit);

            let statements = self.transpile_instruction(label, instruction, arg);

            // println!("{} => {}", label, code_unit_str);
            // for statement in statements.iter() {
            //     println!("\t{}", statement);
            // }

            let prev = if statements.len() == 0 {
                self.blocks.insert(
                    MyLabel::PyLabel(label),
                    StatementBlock {
                        source: Some(code_unit),
                        statements: vec![MyStatement::Noop],
                    },
                )
            } else {
                self.blocks.insert(MyLabel::PyLabel(label), StatementBlock { source: Some(code_unit), statements })
            };

            assert!(prev.is_none());
        }

        StatementObject {
            varname_to_varindex: self.varname_to_varindex,
            varnum_to_varindex: self.varnum_to_varindex,
            constindex_to_varindex: self.constindex_to_varindex,
            blocks: self.blocks,
            varindex_counter: self.varindex_counter,
        }
    }

    fn create_new_variable(&mut self) -> Varindex {
        let varindex_counter = self.varindex_counter;
        self.varindex_counter += 1;
        let varindex = Varindex(varindex_counter);
        self.stack_varindex.push(varindex);
        varindex
    }

    fn create_new_variable_without_stack(&mut self) -> Varindex {
        let varindex_counter = self.varindex_counter;
        self.varindex_counter += 1;
        let varindex = Varindex(varindex_counter);
        varindex
    }

    fn create_jump_placeholder(&mut self) -> usize {
        let jump_placeholder_counter = self.jump_placeholder_counter;
        self.jump_placeholder_counter += 1;
        // let varindex = VarIndex(jump_placeholder_counter);
        self.stack_jump_placeholder.push(jump_placeholder_counter);
        jump_placeholder_counter
    }

    fn transpile_instruction(&mut self, label: usize, instruction: Instruction, arg: OpArg) -> Vec<MyStatement> {
        match instruction {
            Instruction::ExtendedArg => todo!(),
            Instruction::JumpIfFalse { target } => {
                let target = target.get(arg);
                let varindex_rhs = self.pop_varindex();
                let varindex_lhs = self.create_new_variable();
                let target_label = target.0 as usize;

                vec![
                    MyStatement::Assign(varindex_lhs, MyExpression::UnaryOp(UnaryOperator::Not, varindex_rhs)),
                    MyStatement::JumpIfTrue(varindex_lhs, target_label),
                ]
            }
            Instruction::JumpIfTrue { target } => {
                let target = target.get(arg);
                let varindex_rhs = self.pop_varindex();
                let target_label = target.0 as usize;

                vec![MyStatement::JumpIfTrue(varindex_rhs, target_label)]
            }
            Instruction::Jump { target } => {
                let target = target.get(arg);
                let target_label = target.0 as usize;

                vec![MyStatement::Jump(target_label)]
            }
            Instruction::LoadFast(op_arg) => {
                let idx = op_arg.get(arg) as usize;
                let varname = self.code_object.varnames.get(idx).unwrap();
                let varname_str = varname.as_ref().to_string();
                let varindex_rhs = self.varname_to_varindex.get(&varname_str).unwrap().clone();
                self.stack_varindex.push(varindex_rhs);

                vec![]
            }
            Instruction::StoreFast(op_arg) => {
                let idx = op_arg.get(arg) as usize;

                let varindex_lhs = *self.varnum_to_varindex.get(&idx).unwrap();
                let varindex_rhs = self.pop_varindex();

                vec![MyStatement::Assign(varindex_lhs, MyExpression::Variable(varindex_rhs))]
            }
            Instruction::LoadConst { idx } => {
                let constindex = idx.get(arg) as usize;
                let varindex_const = self.constindex_to_varindex.get(&constindex).unwrap().clone();
                self.stack_varindex.push(varindex_const);

                vec![]
            }
            Instruction::BuildTuple { size } => {
                let size = size.get(arg) as usize;

                let mut tuple = Vec::new();
                for i in 0..size {
                    tuple.push(self.pop_varindex())
                }
                let varindex_lhs = self.create_new_variable();

                vec![MyStatement::Assign(varindex_lhs, MyExpression::Struct(tuple))]
            }
            Instruction::UnpackSequence { size } => {
                // self.print();
                todo!()
            }
            Instruction::ReturnValue => {
                let varindex_return = self.pop_varindex();

                vec![MyStatement::Return(varindex_return)]
            }
            Instruction::ReturnConst { idx } => {
                let constindex = idx.get(arg) as usize;

                vec![MyStatement::Return(Varindex(constindex))]
            }
            Instruction::CompareOperation { op, .. } => {
                let op = op.get(arg);

                let b = self.pop_varindex();
                let a = self.pop_varindex();
                let variable_lhs = self.create_new_variable();

                vec![MyStatement::Assign(variable_lhs, MyExpression::CompareOp(op, a, b))]
            }
            Instruction::UnaryOperation { op, .. } => {
                let op = op.get(arg);

                let a = self.pop_varindex();
                let variable_lhs = self.create_new_variable();

                vec![MyStatement::Assign(variable_lhs, MyExpression::UnaryOp(op, a))]
            }
            Instruction::BinaryOperation { op } | Instruction::BinaryOperationInplace { op } => {
                let op = op.get(arg);

                let b = self.pop_varindex();
                let a = self.pop_varindex();
                let variable_lhs = self.create_new_variable();

                vec![MyStatement::Assign(variable_lhs, MyExpression::BinaryOp(op, a, b))]
            }
            Instruction::SetupLoop => {
                vec![]
            }
            Instruction::PopBlock => {
                // TODO(hanif)
                let jump_placeholder = self.pop_jump_placeholder();
                vec![MyStatement::PlaceholderTarget(jump_placeholder)]
            }
            Instruction::LoadGlobal(idx) => {
                let global_idx = idx.get(arg) as usize;
                let varindex_global = self.globalindex_to_varindex.get(&global_idx).unwrap().clone();
                self.stack_varindex.push(varindex_global);
                vec![]
            }
            Instruction::CallFunctionPositional { nargs } => {
                let size = nargs.get(arg) as usize;

                let mut tuple = Vec::new();
                for i in 0..size {
                    tuple.push(self.pop_varindex())
                }
                let varindex_function = self.pop_varindex();
                let varindex_lhs = self.create_new_variable();

                vec![MyStatement::Assign(varindex_lhs, MyExpression::Call(varindex_function, tuple))]
            }
            Instruction::Subscript => {
                let varindex_key = self.pop_varindex();
                let varindex_container = self.pop_varindex();
                let variable_lhs = self.create_new_variable();

                vec![MyStatement::Assign(variable_lhs, MyExpression::Subscript(varindex_container, varindex_key))]
            }
            Instruction::GetIter => {
                // TODO(hanif)
                let varindex_iterator = self.pop_varindex();
                let variable_lhs = self.create_new_variable();
                vec![MyStatement::Assign(variable_lhs, MyExpression::ConvertToIterator(varindex_iterator))]
            }
            Instruction::ForIter { .. } => {
                // TODO(hanif) - This instruction obtains the value from the iterator?
                let varindex_iterator = self.pop_varindex();
                let varindex_none = self.create_new_variable_without_stack();
                let varindex_check = self.create_new_variable_without_stack();
                let varindex_cmp = self.create_new_variable_without_stack();
                let jump_placeholder = self.create_jump_placeholder();
                let varindex_get = self.create_new_variable();

                vec![
                    MyStatement::Assign(varindex_none, MyExpression::Int(0)),
                    MyStatement::Assign(varindex_check, MyExpression::IteratorCheck(varindex_iterator)),
                    MyStatement::Assign(varindex_cmp, MyExpression::CompareOp(ComparisonOperator::Equal, varindex_check, varindex_none)),
                    MyStatement::PlaceholderJump(varindex_cmp, jump_placeholder),
                    MyStatement::Assign(varindex_get, MyExpression::IteratorNext(varindex_iterator)),
                ]
            }
            Instruction::Pop { .. } => {
                // TODO(hanif) - This instruction moves the iterator forward?
                vec![]
            }
            Instruction::Break { .. } => {
                // TODO(hanif) - This instruction moves the iterator forward?
                vec![]
            }
            o => todo!("o:{:?}", o),
        }
    }

    // pub fn get_all_instructions(&self) -> Vec<MyStatement> {
    //     self.blocks.iter().flat_map(|(_, block)| block.statements.iter().cloned()).collect()
    // }

    // pub fn build(self) -> FunctionIr {
    //     FunctionIr {
    //         blocks: self.blocks,
    //         varname_to_varindex: self.varname_to_varindex,
    //     }
    // }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MyType1 {
    Int,
    Float,
    Struct(Vec<Varindex>),
    SolvedStruct(Vec<MyType1>),
    Iterator(String, Box<MyType1>),
    Function(String),
}

impl Display for MyType1 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MyType1::Int => write!(f, "int"),
            MyType1::Float => write!(f, "float"),
            MyType1::Struct(s) => {
                let inner = s.iter().map(|v| v.as_string()).collect::<Vec<_>>().join(", ");
                write!(f, "[{}]", inner)
            }
            MyType1::SolvedStruct(s) => {
                let inner = s.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ");
                write!(f, "[{}]", inner)
            }
            MyType1::Iterator(name, value_type) => write!(f, "iterator {} {}", name, value_type),
            MyType1::Function(name) => write!(f, "function {}", name),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MyType2 {
    Int,
    Float,
    Struct(Vec<MyType2>),
    Iterator(String, Box<MyType2>),
    Function(String),
}

#[derive(Clone)]
pub struct TypeMap {
    pub lhs_types: HashMap<Varindex, MyType2>,
    pub varname_types: HashMap<String, MyType2>,
    pub argument_types: Vec<MyType2>,
    pub return_type: Option<MyType2>,
}

#[derive(Clone)]
pub struct TypeResolver {
    pub varindex_to_types: HashMap<Varindex, MyType1>,
    varname_to_types: HashMap<String, MyType1>,
    // varindex_to_globals: HashMap<Varindex, String>,
    statement_object: StatementObject,
    argument_types: Vec<MyType1>,
    return_types: Option<MyType1>,
    expected_len: usize,
}

impl TypeResolver {
    pub fn new<C>(code_object: &CodeObject<C>, args: &[JitType], statement_object: StatementObject) -> Self
    where
        C: Constant,
    {
        let mut varname_types = HashMap::with_capacity(1024);
        let mut argument_types = Vec::new();
        for (idx, jit_type) in args.iter().enumerate() {
            let key = code_object.varnames.get(idx).unwrap().as_ref().to_string();
            let value = match jit_type {
                JitType::Int => MyType1::Int,
                JitType::Float => MyType1::Float,
                JitType::Bool => MyType1::Int,
            };
            varname_types.insert(key, value.clone());
            argument_types.push(value);
        }
        Self {
            varindex_to_types: HashMap::new(),
            varname_to_types: varname_types,
            // varindex_to_globals: HashMap::new(),
            argument_types,
            return_types: None,
            expected_len: statement_object.varindex_counter(),
            statement_object,
        }
    }

    pub fn solve(&mut self) {
        loop {
            let prev = self.type_count();
            self.solve_statement_object();
            let after = self.type_count();

            println!("solving types prev:{} after:{}", prev, after);

            if prev == after {
                // self.warn_ambiguous_types();

                // TODO(hanif) - Ideally should return error of course.
                if self.varindex_to_types.len() != self.statement_object.varindex_counter() {
                    panic!("Not able to solve all types");
                }
                return;
            }
        }
    }

    fn solve_statement_object(&mut self) {
        for (label, statement_block) in self.statement_object.blocks.clone().iter() {
            for statement in statement_block.statements.iter() {
                self.solve_statement(statement);
            }
        }
    }

    fn solve_statement(&mut self, statement: &MyStatement) {
        match statement {
            MyStatement::Assign(lhs, rhs) => self.solve_statement_assignment(*lhs, rhs),
            MyStatement::Return(lhs) => {
                let my_type = self.varindex_to_types.get(lhs).cloned();
                self.lock_type_and_check(*lhs, my_type)
            }
            MyStatement::Jump(_) => {
                // TODO(hanif) - Nothing to solve?
            }
            MyStatement::JumpIfTrue(_, _) => {
                // TODO(hanif) - Nothing to solve?
            }
            MyStatement::PlaceholderJump(_, _) => {
                // TODO(hanif) - Nothing to solve?
            }
            MyStatement::PlaceholderTarget(_) => {
                // TODO(hanif) - Nothing to solve?
            }
            MyStatement::Noop => {
                // TODO(hanif) - Nothing to solve?
            }
        }
    }

    fn solve_statement_assignment(&mut self, lhs: Varindex, rhs: &MyExpression) {
        match rhs {
            MyExpression::Varname(s) => {
                self.lock_type_and_check(lhs, self.varname_to_types.get(s).cloned());
            }
            MyExpression::Variable(varindex) => {
                let my_type = self.varindex_to_types.get(varindex);
                self.lock_type_and_check(lhs, my_type.cloned());
            }
            MyExpression::Int(_) => {
                self.lock_type_and_check(lhs, Some(MyType1::Int));
            }
            MyExpression::Float64(_) => {
                self.lock_type_and_check(lhs, Some(MyType1::Float));
            }
            MyExpression::Struct(s) => {
                self.lock_type_and_check(lhs, Some(MyType1::Struct(s.clone())));
            }
            MyExpression::UnaryOp(_, a) => {
                let types_a = self.varindex_to_types.get(a).cloned();
                if let Some(types_a) = types_a {
                    match types_a {
                        MyType1::Int => {
                            self.lock_type_and_check(lhs, Some(MyType1::Int));
                        }
                        MyType1::Float => {
                            self.lock_type_and_check(lhs, Some(MyType1::Float));
                        }
                        _ => {
                            self.print();
                            todo!()
                        }
                    };
                }
            }
            MyExpression::BinaryOp(op, a, b) => {
                let types_a = self.varindex_to_types.get(a);
                let types_b = self.varindex_to_types.get(b);
                if let (Some(types_a), Some(types_b)) = (types_a, types_b) {
                    match (op, types_a, types_b) {
                        (_, MyType1::Int, MyType1::Int) => {
                            self.lock_type_and_check(lhs, Some(MyType1::Int));
                        }
                        (_, _, MyType1::Float) => {
                            self.lock_type_and_check(lhs, Some(MyType1::Float));
                        }
                        (_, MyType1::Float, _) => {
                            self.lock_type_and_check(lhs, Some(MyType1::Float));
                        }
                        _ => {
                            self.print();
                            todo!()
                        }
                    };
                }
            }
            MyExpression::CompareOp(_, _, _) => {
                self.lock_type_and_check(lhs, Some(MyType1::Int));
            }
            MyExpression::Subscript(container, key) => {
                let container_types = self.varindex_to_types.get(container).unwrap();
                let inner_type = match container_types {
                    MyType1::Struct(s) => {
                        let constant = self.get_constant_for_lhs(key);
                        match constant {
                            Some(x) => match x {
                                MyExpression::Int(value) => s.get(value as usize).cloned().unwrap(),
                                _ => {
                                    todo!()
                                }
                            },
                            None => {
                                todo!()
                            }
                        }
                    }
                    _ => {
                        todo!()
                    }
                };
                let possible_types = self.varindex_to_types.get(&inner_type);
                self.lock_type_and_check(lhs, possible_types.cloned())
            }
            MyExpression::GlobalFunction(name) => {
                let my_type = MyType1::Function(name.clone());
                self.lock_type_and_check(lhs, Some(my_type));
            }
            MyExpression::Call(varindex_func, varindex_args) => {
                self.solve_function_call(lhs, *varindex_func, varindex_args);
            }
            MyExpression::ConvertToIterator(varindex_iterator) => {
                // TODO(hanif) - Just assert that the variable is a tuple of 3 things.
                //  And the 2nd and 3rd variables be integers.
                let my_type = self.varindex_to_types.get(varindex_iterator);
                if let Some(my_type) = my_type {
                    let resolved_type = match my_type {
                        MyType1::Iterator(name, next_type) => MyType1::Iterator(name.clone(), next_type.clone()),
                        _ => {
                            self.print();
                            todo!()
                        }
                    };
                    self.lock_type_and_check(lhs, Some(resolved_type));
                }
            }
            MyExpression::IteratorNext(varindex_iterator) => {
                let my_type = self.varindex_to_types.get(varindex_iterator);
                if let Some(my_type) = my_type {
                    let resolved_type = match my_type {
                        MyType1::Iterator(name, next_type) => next_type.as_ref().clone(),
                        _ => {
                            self.print();
                            todo!()
                        }
                    };
                    self.lock_type_and_check(lhs, Some(resolved_type));
                }
            }
            MyExpression::IteratorCheck(varindex_iterator) => {
                self.lock_type_and_check(lhs, Some(MyType1::Int));
            }
            MyExpression::String(_) => {
                self.print();
                todo!()
            }
        }
    }

    fn solve_function_call(&mut self, lhs: Varindex, varindex_func: Varindex, varindex_args: &Vec<Varindex>) {
        let name = self.varindex_to_types.get(&varindex_func).unwrap();
        if let MyType1::Function(name) = name {
            match name.as_str() {
                "range" => match varindex_args.len() {
                    1 => {
                        let my_type = MyType1::Iterator("range".to_string(), Box::new(MyType1::Int));
                        self.lock_type_and_check(lhs, Some(my_type));
                    }
                    _ => {
                        self.print();
                        todo!()
                    }
                },
                _ => {
                    self.print();
                    todo!()
                }
            }
        } else {
            self.print();
            todo!()
        }
    }

    fn lock_type_and_check(&mut self, lhs: Varindex, my_type: Option<MyType1>) {
        if let Some(my_type) = my_type {
            let prev = self.varindex_to_types.insert(lhs, my_type.clone());
            if let Some(prev_type) = prev {
                assert_eq!(prev_type, my_type);
            }
        }
    }

    fn type_count(&self) -> usize {
        self.varindex_to_types.len() + self.varname_to_types.len() + self.return_types.as_ref().map(|_| 1).unwrap_or(0)
    }

    pub fn print(&self) {
        let mut varindex_types = self.varindex_to_types.iter().map(|s| (s.0.clone(), s.1.clone())).collect::<Vec<_>>();
        varindex_types.sort_by(|a, b| a.0.cmp(&b.0));
        let return_type = self.return_types.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ");

        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
        for idx in 0..self.expected_len {
            let varindex = Varindex(idx);
            let my_type = self.varindex_to_types.get(&varindex).cloned();
            match my_type {
                Some(my_type) => {
                    println!("{} => {}", varindex.as_string(), my_type);
                }
                None => {
                    println!("{} => unknown", varindex.as_string());
                }
            }
        }
        println!("return types => [{return_type}]",);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
    }

    // fn warn_ambiguous_types(&self) {
    //     for (lhs, varindex_types) in self.varindex_to_types.iter() {
    //         if varindex_types.len() > 1 {
    //             let varindex_types_str = varindex_types.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ");
    //             println!("***********************");
    //             println!("{} have ambiguous types => {}", lhs.as_string(), varindex_types_str);
    //             println!("***********************");
    //         }
    //     }
    //     if self.return_types.len() > 1 {
    //         let return_types_str = self.return_types.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ");
    //         println!("***********************");
    //         println!("return value has ambiguous types => [{}]", return_types_str);
    //         println!("***********************");
    //     }
    // }

    fn get_constant_for_lhs(&self, lhs: &Varindex) -> Option<MyExpression> {
        for (label, statement_block) in self.statement_object.blocks.iter() {
            for statement in statement_block.statements.iter() {
                match statement {
                    MyStatement::Assign(l, r) => {
                        if l == lhs {
                            let result = match r {
                                MyExpression::Varname(_) => None,
                                MyExpression::Variable(varindex) => self.get_constant_for_lhs(varindex),
                                MyExpression::Int(_) => Some(r.clone()),
                                MyExpression::Float64(_) => Some(r.clone()),
                                MyExpression::Struct(_) => None,
                                MyExpression::UnaryOp(_, _) => Some(r.clone()),
                                MyExpression::BinaryOp(_, _, _) => Some(r.clone()),
                                MyExpression::CompareOp(_, _, _) => Some(r.clone()),
                                MyExpression::Subscript(_, _) => Some(r.clone()),
                                MyExpression::GlobalFunction(_) => Some(r.clone()),
                                MyExpression::Call(_, _) => Some(r.clone()),
                                _ => {
                                    todo!()
                                }
                            };
                            return result;
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }

    fn upgrade_type_1_to_type_2(&self, type_1: &MyType1) -> MyType2 {
        match type_1 {
            MyType1::Int => MyType2::Int,
            MyType1::Float => MyType2::Float,
            MyType1::Struct(s) => {
                let mut rs = Vec::with_capacity(s.len());
                for lhs in s.iter() {
                    let member_type1 = self.varindex_to_types.get(lhs).unwrap();
                    rs.push(self.upgrade_type_1_to_type_2(member_type1));
                }
                MyType2::Struct(rs)
            }
            MyType1::SolvedStruct(s) => MyType2::Struct(s.iter().map(|t| self.upgrade_type_1_to_type_2(t)).collect()),
            MyType1::Iterator(name, value_type) => {
                let name = name.clone();
                let value_type = Box::new(self.upgrade_type_1_to_type_2(value_type.as_ref()).clone());
                MyType2::Iterator(name, value_type)
            }
            MyType1::Function(name) => MyType2::Function(name.clone()),
        }
    }

    pub fn build(self) -> TypeMap {
        let lhs_types = self.varindex_to_types.iter().map(|s| (s.0.clone(), self.upgrade_type_1_to_type_2(s.1))).collect::<HashMap<_, _>>();
        let varname_types = self.varname_to_types.iter().map(|(n, t)| (n.clone(), self.upgrade_type_1_to_type_2(&t))).collect::<HashMap<_, _>>();
        let argument_types = self.argument_types.iter().map(|t| self.upgrade_type_1_to_type_2(&t)).collect();
        let return_type = self.return_types.iter().next().map(|t| self.upgrade_type_1_to_type_2(t));
        TypeMap {
            lhs_types,
            varname_types,
            argument_types,
            return_type,
        }
    }
}

pub fn stringify_code_object(code_unit: CodeUnit) -> String {
    macro_rules! w {
        ($variant:ident) => {
            format!("{} {}", stringify!($variant), code_unit.arg.0)
        };
    }

    match code_unit.op {
        Instruction::Nop => w!(Nop),
        Instruction::ImportName { idx } => w!(ImportName),
        Instruction::ImportNameless => w!(ImportNameless),
        Instruction::ImportStar => w!(ImportStar),
        Instruction::ImportFrom { idx } => w!(ImportFrom),
        Instruction::LoadFast(idx) => w!(LoadFast),
        Instruction::LoadNameAny(idx) => w!(LoadNameAny),
        Instruction::LoadGlobal(idx) => w!(LoadGlobal),
        Instruction::LoadDeref(idx) => w!(LoadDeref),
        Instruction::LoadClassDeref(idx) => w!(LoadClassDeref),
        Instruction::StoreFast(idx) => w!(StoreFast),
        Instruction::StoreLocal(idx) => w!(StoreLocal),
        Instruction::StoreGlobal(idx) => w!(StoreGlobal),
        Instruction::StoreDeref(idx) => w!(StoreDeref),
        Instruction::DeleteFast(idx) => w!(DeleteFast),
        Instruction::DeleteLocal(idx) => w!(DeleteLocal),
        Instruction::DeleteGlobal(idx) => w!(DeleteGlobal),
        Instruction::DeleteDeref(idx) => w!(DeleteDeref),
        Instruction::LoadClosure(i) => w!(LoadClosure),
        Instruction::Subscript => w!(Subscript),
        Instruction::StoreSubscript => w!(StoreSubscript),
        Instruction::DeleteSubscript => w!(DeleteSubscript),
        Instruction::StoreAttr { idx } => w!(StoreAttr),
        Instruction::DeleteAttr { idx } => w!(DeleteAttr),
        Instruction::LoadConst { idx } => w!(LoadConst),
        Instruction::UnaryOperation { op } => w!(UnaryOperation),
        Instruction::BinaryOperation { op } => w!(BinaryOperation),
        Instruction::BinaryOperationInplace { op } => w!(BinaryOperationInplace),
        Instruction::BinarySubscript => w!(BinarySubscript),
        Instruction::LoadAttr { idx } => w!(LoadAttr),
        Instruction::TestOperation { op } => w!(TestOperation),
        Instruction::CompareOperation { op } => w!(CompareOperation),
        Instruction::CopyItem { index } => w!(CopyItem),
        Instruction::Pop => w!(Pop),
        Instruction::Swap { index } => w!(Swap),
        // ToBool => w!(ToBool),
        Instruction::Rotate2 => w!(Rotate2),
        Instruction::Rotate3 => w!(Rotate3),
        Instruction::Duplicate => w!(Duplicate),
        Instruction::Duplicate2 => w!(Duplicate2),
        Instruction::GetIter => w!(GetIter),
        // GET_LEN
        Instruction::GetLen => w!(GetLen),
        Instruction::Continue { target } => w!(Continue),
        Instruction::Break { target } => w!(Break),
        Instruction::Jump { target } => w!(Jump),
        Instruction::JumpIfTrue { target } => w!(JumpIfTrue),
        Instruction::JumpIfFalse { target } => w!(JumpIfFalse),
        Instruction::JumpIfTrueOrPop { target } => w!(JumpIfTrueOrPop),
        Instruction::JumpIfFalseOrPop { target } => w!(JumpIfFalseOrPop),
        Instruction::MakeFunction(flags) => w!(MakeFunction),
        Instruction::CallFunctionPositional { nargs } => w!(CallFunctionPositional),
        Instruction::CallFunctionKeyword { nargs } => w!(CallFunctionKeyword),
        Instruction::CallFunctionEx { has_kwargs } => w!(CallFunctionEx),
        Instruction::LoadMethod { idx } => w!(LoadMethod),
        Instruction::CallMethodPositional { nargs } => w!(CallMethodPositional),
        Instruction::CallMethodKeyword { nargs } => w!(CallMethodKeyword),
        Instruction::CallMethodEx { has_kwargs } => w!(CallMethodEx),
        Instruction::ForIter { target } => w!(ForIter),
        Instruction::ReturnValue => w!(ReturnValue),
        Instruction::ReturnConst { idx } => w!(ReturnConst),
        Instruction::YieldValue => w!(YieldValue),
        Instruction::YieldFrom => w!(YieldFrom),
        Instruction::SetupAnnotation => w!(SetupAnnotation),
        Instruction::SetupLoop => w!(SetupLoop),
        Instruction::SetupExcept { handler } => w!(SetupExcept),
        Instruction::SetupFinally { handler } => w!(SetupFinally),
        Instruction::EnterFinally => w!(EnterFinally),
        Instruction::EndFinally => w!(EndFinally),
        Instruction::SetupWith { end } => w!(SetupWith),
        Instruction::WithCleanupStart => w!(WithCleanupStart),
        Instruction::WithCleanupFinish => w!(WithCleanupFinish),
        Instruction::BeforeAsyncWith => w!(BeforeAsyncWith),
        Instruction::SetupAsyncWith { end } => w!(SetupAsyncWith),
        Instruction::PopBlock => w!(PopBlock),
        Instruction::Raise { kind } => w!(Raise),
        Instruction::BuildString { size } => w!(BuildString),
        Instruction::BuildTuple { size } => w!(BuildTuple),
        Instruction::BuildTupleFromTuples { size } => w!(BuildTupleFromTuples),
        Instruction::BuildTupleFromIter => w!(BuildTupleFromIter),
        Instruction::BuildList { size } => w!(BuildList),
        Instruction::BuildListFromTuples { size } => w!(BuildListFromTuples),
        Instruction::BuildSet { size } => w!(BuildSet),
        Instruction::BuildSetFromTuples { size } => w!(BuildSetFromTuples),
        Instruction::BuildMap { size } => w!(BuildMap),
        Instruction::BuildMapForCall { size } => w!(BuildMapForCall),
        Instruction::DictUpdate => w!(DictUpdate),
        Instruction::BuildSlice { step } => w!(BuildSlice),
        Instruction::ListAppend { i } => w!(ListAppend),
        Instruction::SetAdd { i } => w!(SetAdd),
        Instruction::MapAdd { i } => w!(MapAdd),
        Instruction::PrintExpr => w!(PrintExpr),
        Instruction::LoadBuildClass => w!(LoadBuildClass),
        Instruction::UnpackSequence { size } => w!(UnpackSequence),
        Instruction::UnpackEx { args } => w!(UnpackEx),
        Instruction::FormatValue { conversion } => w!(FormatValue),
        Instruction::PopException => w!(PopException),
        Instruction::Reverse { amount } => w!(Reverse),
        Instruction::GetAwaitable => w!(GetAwaitable),
        Instruction::GetAIter => w!(GetAIter),
        Instruction::GetANext => w!(GetANext),
        Instruction::EndAsyncFor => w!(EndAsyncFor),
        Instruction::MatchMapping => w!(MatchMapping),
        Instruction::MatchSequence => w!(MatchSequence),
        Instruction::MatchKeys => w!(MatchKeys),
        Instruction::MatchClass(arg) => w!(MatchClass),
        Instruction::ExtendedArg => w!(ExtendedArg),
        Instruction::TypeVar => w!(TypeVar),
        Instruction::TypeVarWithBound => w!(TypeVarWithBound),
        Instruction::TypeVarWithConstraint => w!(TypeVarWithConstraint),
        Instruction::TypeAlias => w!(TypeAlias),
        Instruction::ParamSpec => w!(ParamSpec),
        Instruction::TypeVarTuple => w!(TypeVarTuple),
    }
}
