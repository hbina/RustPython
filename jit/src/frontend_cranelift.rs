// use crate::JitType;
// use cranelift::codegen::ir::StackSlot;
// use cranelift::codegen::ir::stackslot::StackSize;
// use cranelift::codegen::isa::TargetFrontendConfig;
// use cranelift::frontend::{FunctionBuilder, Variable};
// use cranelift::prelude::{Block, EntityRef, InstBuilder, StackSlotData, StackSlotKind, Type, types};
// use num_traits::ToPrimitive;
// use rustpython_compiler_core::bytecode::{BinaryOperator, BorrowedConstant, CodeObject, CodeUnit, ComparisonOperator, Constant, Instruction, OpArg, OpArgState, UnaryOperator};
// use std::collections::{BTreeMap, HashMap, HashSet};
// use std::fmt::{Display, Formatter};
// use crate::transpiler::{FunctionIr, MyExpression, MyType2, TypeMap, Varindex};
//
// pub struct FunctionIrCompiler<'a, 'b> {
//     builder: &'a mut FunctionBuilder<'b>,
//     target_config: TargetFrontendConfig,
//     varindex_to_variable: HashMap<Varindex, Variable>,
//     varindex_to_stack_slot: HashMap<Varindex, (StackSlot, Vec<MyType2>)>,
//     label_to_block: HashMap<usize, Block>,
//     type_map: TypeMap, // pub(crate) sig: JitSig,
//     function_ir: FunctionIr,
// }
//
// impl<'a, 'b> FunctionIrCompiler<'a, 'b> {
//     pub fn new(builder: &'a mut FunctionBuilder<'b>, target_config: TargetFrontendConfig, function_ir: FunctionIr, type_map: TypeMap) -> FunctionIrCompiler<'a, 'b> {
//         // Create all the variables that the function uses all at once
//         let lhs_types = type_map.lhs_types.iter().map(|s| (s.0.clone(), s.1.clone())).collect::<HashMap<_, _>>();
//
//         let mut label_to_block = HashMap::new();
//
//         for (label, my_block) in function_ir.blocks.iter() {
//             label_to_block.insert(*label, builder.create_block());
//         }
//
//         let mut result = FunctionIrCompiler {
//             builder,
//             target_config,
//             varindex_to_variable: HashMap::new(),
//             varindex_to_stack_slot: HashMap::new(),
//             label_to_block,
//             type_map,
//             function_ir,
//         };
//
//         for (varindex, my_type) in lhs_types.iter() {
//             result.create_variable(varindex, my_type);
//         }
//
//         result
//     }
//
//     fn create_variable(&mut self, lhs: &Varindex, my_type: &MyType2) {
//         let builder = &mut self.builder;
//
//         match my_type {
//             MyType2::Int => {
//                 let variable = Variable::new(lhs.0);
//                 self.varindex_to_variable.insert(lhs.clone(), variable);
//                 builder.declare_var(variable, my_type.to_cranelift());
//             }
//             MyType2::Float => {
//                 let variable = Variable::new(lhs.0);
//                 self.varindex_to_variable.insert(lhs.clone(), variable);
//                 builder.declare_var(variable, my_type.to_cranelift());
//             }
//             MyType2::Struct(s) => {
//                 let stack_slot_data = StackSlotData::new(StackSlotKind::ExplicitSlot, my_type.stack_size(), 0);
//                 let stack_slot = builder.create_sized_stack_slot(stack_slot_data);
//                 self.varindex_to_stack_slot.insert(*lhs, (stack_slot, s.clone()));
//             }
//             _ => {
//                 todo!()
//             }
//         }
//     }
//
//     // fn get_or_create_block(&mut self, label: usize) -> Block {
//     //     let builder = &mut self.builder;
//     //     *self.label_to_block.entry(label).or_insert_with(|| builder.create_block())
//     // }
//
//     pub fn compile(&mut self) {
//         let blocks = self.function_ir.blocks.iter().map(|s| (s.0.clone(), s.1.clone())).collect::<HashMap<_, _>>();
//
//         println!("=========================");
//         for (label, my_block) in blocks.iter() {
//             let builder_block = self.label_to_block.get_mut(label).unwrap();
//             // let mut block = self.get_or_create_block(*label);
//
//             // The 0th label is always the entry of the function
//             if *label == 0 {
//                 self.builder.append_block_params_for_function_params(*builder_block);
//                 self.builder.switch_to_block(*builder_block);
//             }
//
//             todo!()
//             // println!("block:{}", label);
//             // for statement in my_block.statements.iter() {
//             //     println!("\t{}", statement.as_string());
//             //     match statement {
//             //         MyStatement::Assign(lhs, rhs) => {
//             //             self.compile_statement_assignment(lhs, rhs);
//             //         }
//             //         MyStatement::Jump(j) => {}
//             //         MyStatement::Return(r) => {}
//             //     }
//             // }
//         }
//     }
//
//     fn compile_statement_assignment(&mut self, varindex_lhs: &Varindex, rhs: &MyExpression) {
//         match rhs {
//             MyExpression::Varname(v) => {
//                 let varindex_rhs = self.function_ir.varname_to_varindex.get(v).unwrap();
//
//                 match (
//                     self.varindex_to_variable.get(varindex_lhs),
//                     self.varindex_to_stack_slot.get(varindex_lhs),
//                     self.varindex_to_variable.get(varindex_rhs),
//                     self.varindex_to_stack_slot.get(varindex_rhs),
//                 ) {
//                     (Some(variable_lhs), None, Some(variable_rhs), None) => {
//                         let value_rhs = self.builder.try_use_var(*variable_rhs).unwrap();
//                         self.builder.try_def_var(*variable_lhs, value_rhs).unwrap();
//                     }
//                     (None, Some((stack_slot_lhs, struct_lhs)), None, Some((stack_slot_rhs, struct_rhs))) => {
//                         let stack_size_lhs = struct_lhs.iter().map(|s| s.stack_size()).sum::<StackSize>();
//                         let stack_size_rhs = struct_rhs.iter().map(|s| s.stack_size()).sum::<StackSize>();
//
//                         assert_eq!(stack_size_lhs, stack_size_rhs);
//                         let value_dst = self.builder.ins().stack_addr(Type::int_with_byte_size(1).unwrap(), *stack_slot_lhs, 0);
//                         let value_src = self.builder.ins().stack_addr(Type::int_with_byte_size(1).unwrap(), *stack_slot_rhs, 0);
//                         let value_size = self.builder.ins().iconst(types::I64, stack_size_lhs as i64);
//
//                         self.builder.call_memcpy(self.target_config.clone(), value_dst, value_src, value_size);
//                     }
//                     _ => {
//                         todo!()
//                     }
//                 }
//             }
//             MyExpression::Variable(varindex_rhs) => {
//                 match (
//                     self.varindex_to_variable.get(varindex_lhs),
//                     self.varindex_to_stack_slot.get(varindex_lhs),
//                     self.varindex_to_variable.get(varindex_rhs),
//                     self.varindex_to_stack_slot.get(varindex_rhs),
//                 ) {
//                     (Some(variable_lhs), None, Some(variable_rhs), None) => {
//                         let value_rhs = self.builder.try_use_var(*variable_rhs).unwrap();
//                         self.builder.try_def_var(*variable_lhs, value_rhs).unwrap();
//                     }
//                     (None, Some((stack_slot_lhs, struct_lhs)), None, Some((stack_slot_rhs, struct_rhs))) => {
//                         let stack_size_lhs = struct_lhs.iter().map(|s| s.stack_size()).sum::<StackSize>();
//                         let stack_size_rhs = struct_rhs.iter().map(|s| s.stack_size()).sum::<StackSize>();
//
//                         assert_eq!(stack_size_lhs, stack_size_rhs);
//                         let value_dst = self.builder.ins().stack_addr(Type::int_with_byte_size(1).unwrap(), *stack_slot_lhs, 0);
//                         let value_src = self.builder.ins().stack_addr(Type::int_with_byte_size(1).unwrap(), *stack_slot_rhs, 0);
//                         let value_size = self.builder.ins().iconst(types::I64, stack_size_lhs as i64);
//
//                         self.builder.call_memcpy(self.target_config.clone(), value_dst, value_src, value_size);
//                     }
//                     _ => {
//                         todo!()
//                     }
//                 }
//             }
//             MyExpression::Int(v) => {
//                 let builder = &mut self.builder;
//
//                 let variable_lhs = self.varindex_to_variable.get(varindex_lhs).unwrap();
//
//                 let rhs_value = builder.ins().iconst(types::I64, *v);
//                 builder.try_def_var(*variable_lhs, rhs_value).unwrap();
//             }
//             MyExpression::Float64(v) => {
//                 let builder = &mut self.builder;
//
//                 let variable_lhs = self.varindex_to_variable.get(varindex_lhs).unwrap();
//
//                 let rhs_value = builder.ins().f64const(*v);
//                 builder.try_def_var(*variable_lhs, rhs_value).unwrap();
//             }
//             MyExpression::Struct(s) => {
//                 // no-op as we already created the stack at the top of the function
//             }
//             MyExpression::UnaryOp(_, _) => {
//                 // TODO(hanif)
//             }
//             MyExpression::BinaryOp(_, _, _) => {
//                 // TODO(hanif)
//             }
//             MyExpression::CompareOp(_, _, _) => {
//                 // TODO(hanif)
//             }
//             MyExpression::Subscript(_, _) => {
//                 // TODO(hanif)
//             }
//             MyExpression::GlobalFunction(_) => {
//                 // TODO(hanif)
//             }
//             MyExpression::Call(_, _) => {
//                 // TODO(hanif)
//             }
//             _ => {
//                 todo!()
//             }
//         }
//     }
// }
//
// pub fn stringify_code_object(code_unit: CodeUnit) -> String {
//     macro_rules! w {
//         ($variant:ident) => {
//             format!("{} {}", stringify!($variant), code_unit.arg.0)
//         };
//     }
//
//     match code_unit.op {
//         Instruction::Nop => w!(Nop),
//         Instruction::ImportName { idx } => w!(ImportName),
//         Instruction::ImportNameless => w!(ImportNameless),
//         Instruction::ImportStar => w!(ImportStar),
//         Instruction::ImportFrom { idx } => w!(ImportFrom),
//         Instruction::LoadFast(idx) => w!(LoadFast),
//         Instruction::LoadNameAny(idx) => w!(LoadNameAny),
//         Instruction::LoadGlobal(idx) => w!(LoadGlobal),
//         Instruction::LoadDeref(idx) => w!(LoadDeref),
//         Instruction::LoadClassDeref(idx) => w!(LoadClassDeref),
//         Instruction::StoreFast(idx) => w!(StoreFast),
//         Instruction::StoreLocal(idx) => w!(StoreLocal),
//         Instruction::StoreGlobal(idx) => w!(StoreGlobal),
//         Instruction::StoreDeref(idx) => w!(StoreDeref),
//         Instruction::DeleteFast(idx) => w!(DeleteFast),
//         Instruction::DeleteLocal(idx) => w!(DeleteLocal),
//         Instruction::DeleteGlobal(idx) => w!(DeleteGlobal),
//         Instruction::DeleteDeref(idx) => w!(DeleteDeref),
//         Instruction::LoadClosure(i) => w!(LoadClosure),
//         Instruction::Subscript => w!(Subscript),
//         Instruction::StoreSubscript => w!(StoreSubscript),
//         Instruction::DeleteSubscript => w!(DeleteSubscript),
//         Instruction::StoreAttr { idx } => w!(StoreAttr),
//         Instruction::DeleteAttr { idx } => w!(DeleteAttr),
//         Instruction::LoadConst { idx } => w!(LoadConst),
//         Instruction::UnaryOperation { op } => w!(UnaryOperation),
//         Instruction::BinaryOperation { op } => w!(BinaryOperation),
//         Instruction::BinaryOperationInplace { op } => w!(BinaryOperationInplace),
//         Instruction::BinarySubscript => w!(BinarySubscript),
//         Instruction::LoadAttr { idx } => w!(LoadAttr),
//         Instruction::TestOperation { op } => w!(TestOperation),
//         Instruction::CompareOperation { op } => w!(CompareOperation),
//         Instruction::CopyItem { index } => w!(CopyItem),
//         Instruction::Pop => w!(Pop),
//         Instruction::Swap { index } => w!(Swap),
//         // ToBool => w!(ToBool),
//         Instruction::Rotate2 => w!(Rotate2),
//         Instruction::Rotate3 => w!(Rotate3),
//         Instruction::Duplicate => w!(Duplicate),
//         Instruction::Duplicate2 => w!(Duplicate2),
//         Instruction::GetIter => w!(GetIter),
//         // GET_LEN
//         Instruction::GetLen => w!(GetLen),
//         Instruction::Continue { target } => w!(Continue),
//         Instruction::Break { target } => w!(Break),
//         Instruction::Jump { target } => w!(Jump),
//         Instruction::JumpIfTrue { target } => w!(JumpIfTrue),
//         Instruction::JumpIfFalse { target } => w!(JumpIfFalse),
//         Instruction::JumpIfTrueOrPop { target } => w!(JumpIfTrueOrPop),
//         Instruction::JumpIfFalseOrPop { target } => w!(JumpIfFalseOrPop),
//         Instruction::MakeFunction(flags) => w!(MakeFunction),
//         Instruction::CallFunctionPositional { nargs } => w!(CallFunctionPositional),
//         Instruction::CallFunctionKeyword { nargs } => w!(CallFunctionKeyword),
//         Instruction::CallFunctionEx { has_kwargs } => w!(CallFunctionEx),
//         Instruction::LoadMethod { idx } => w!(LoadMethod),
//         Instruction::CallMethodPositional { nargs } => w!(CallMethodPositional),
//         Instruction::CallMethodKeyword { nargs } => w!(CallMethodKeyword),
//         Instruction::CallMethodEx { has_kwargs } => w!(CallMethodEx),
//         Instruction::ForIter { target } => w!(ForIter),
//         Instruction::ReturnValue => w!(ReturnValue),
//         Instruction::ReturnConst { idx } => w!(ReturnConst),
//         Instruction::YieldValue => w!(YieldValue),
//         Instruction::YieldFrom => w!(YieldFrom),
//         Instruction::SetupAnnotation => w!(SetupAnnotation),
//         Instruction::SetupLoop => w!(SetupLoop),
//         Instruction::SetupExcept { handler } => w!(SetupExcept),
//         Instruction::SetupFinally { handler } => w!(SetupFinally),
//         Instruction::EnterFinally => w!(EnterFinally),
//         Instruction::EndFinally => w!(EndFinally),
//         Instruction::SetupWith { end } => w!(SetupWith),
//         Instruction::WithCleanupStart => w!(WithCleanupStart),
//         Instruction::WithCleanupFinish => w!(WithCleanupFinish),
//         Instruction::BeforeAsyncWith => w!(BeforeAsyncWith),
//         Instruction::SetupAsyncWith { end } => w!(SetupAsyncWith),
//         Instruction::PopBlock => w!(PopBlock),
//         Instruction::Raise { kind } => w!(Raise),
//         Instruction::BuildString { size } => w!(BuildString),
//         Instruction::BuildTuple { size } => w!(BuildTuple),
//         Instruction::BuildTupleFromTuples { size } => w!(BuildTupleFromTuples),
//         Instruction::BuildTupleFromIter => w!(BuildTupleFromIter),
//         Instruction::BuildList { size } => w!(BuildList),
//         Instruction::BuildListFromTuples { size } => w!(BuildListFromTuples),
//         Instruction::BuildSet { size } => w!(BuildSet),
//         Instruction::BuildSetFromTuples { size } => w!(BuildSetFromTuples),
//         Instruction::BuildMap { size } => w!(BuildMap),
//         Instruction::BuildMapForCall { size } => w!(BuildMapForCall),
//         Instruction::DictUpdate => w!(DictUpdate),
//         Instruction::BuildSlice { step } => w!(BuildSlice),
//         Instruction::ListAppend { i } => w!(ListAppend),
//         Instruction::SetAdd { i } => w!(SetAdd),
//         Instruction::MapAdd { i } => w!(MapAdd),
//         Instruction::PrintExpr => w!(PrintExpr),
//         Instruction::LoadBuildClass => w!(LoadBuildClass),
//         Instruction::UnpackSequence { size } => w!(UnpackSequence),
//         Instruction::UnpackEx { args } => w!(UnpackEx),
//         Instruction::FormatValue { conversion } => w!(FormatValue),
//         Instruction::PopException => w!(PopException),
//         Instruction::Reverse { amount } => w!(Reverse),
//         Instruction::GetAwaitable => w!(GetAwaitable),
//         Instruction::GetAIter => w!(GetAIter),
//         Instruction::GetANext => w!(GetANext),
//         Instruction::EndAsyncFor => w!(EndAsyncFor),
//         Instruction::MatchMapping => w!(MatchMapping),
//         Instruction::MatchSequence => w!(MatchSequence),
//         Instruction::MatchKeys => w!(MatchKeys),
//         Instruction::MatchClass(arg) => w!(MatchClass),
//         Instruction::ExtendedArg => w!(ExtendedArg),
//         Instruction::TypeVar => w!(TypeVar),
//         Instruction::TypeVarWithBound => w!(TypeVarWithBound),
//         Instruction::TypeVarWithConstraint => w!(TypeVarWithConstraint),
//         Instruction::TypeAlias => w!(TypeAlias),
//         Instruction::ParamSpec => w!(ParamSpec),
//         Instruction::TypeVarTuple => w!(TypeVarTuple),
//     }
// }
