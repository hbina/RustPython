use crate::{
    AsObject, Py, PyObjectRef, PyResult, TryFromObject, VirtualMachine,
    builtins::{PyBaseExceptionRef, PyDict, PyDictRef, PyFunction},
    bytecode::CodeFlags,
};
use rustpython_jit::JitType;

pub fn new_jit_error(msg: String, vm: &VirtualMachine) -> PyBaseExceptionRef {
    let jit_error = vm.ctx.exceptions.jit_error.to_owned();
    vm.new_exception_msg(jit_error, msg)
}

fn get_jit_arg_type(dict: &Py<PyDict>, name: &str, vm: &VirtualMachine) -> PyResult<JitType> {
    if let Some(value) = dict.get_item_opt(name, vm)? {
        if value.is(vm.ctx.types.int_type) {
            Ok(JitType::Int)
        } else if value.is(vm.ctx.types.float_type) {
            Ok(JitType::Float)
        } else if value.is(vm.ctx.types.bool_type) {
            Ok(JitType::Bool)
        } else {
            Err(new_jit_error(
                "Jit requires argument to be either int, float or bool".to_owned(),
                vm,
            ))
        }
    } else {
        Err(new_jit_error(
            format!("argument {name} needs annotation"),
            vm,
        ))
    }
}

pub fn get_jit_arg_types(func: &Py<PyFunction>, vm: &VirtualMachine) -> PyResult<Vec<JitType>> {
    let code = func.code.lock();
    let arg_names = code.arg_names();

    if code
        .flags
        .intersects(CodeFlags::HAS_VARARGS | CodeFlags::HAS_VARKEYWORDS)
    {
        return Err(new_jit_error(
            "Can't jit functions with variable number of arguments".to_owned(),
            vm,
        ));
    }

    if arg_names.args.is_empty() && arg_names.kwonlyargs.is_empty() {
        return Ok(Vec::new());
    }

    let func_obj: PyObjectRef = func.as_ref().to_owned();
    let annotations = func_obj.get_attr("__annotations__", vm)?;
    if vm.is_none(&annotations) {
        Err(new_jit_error(
            "Jitting function requires arguments to have annotations".to_owned(),
            vm,
        ))
    } else if let Ok(dict) = PyDictRef::try_from_object(vm, annotations) {
        let mut arg_types = Vec::new();

        for arg in arg_names.args {
            arg_types.push(get_jit_arg_type(&dict, arg.as_str(), vm)?);
        }

        for arg in arg_names.kwonlyargs {
            arg_types.push(get_jit_arg_type(&dict, arg.as_str(), vm)?);
        }

        Ok(arg_types)
    } else {
        Err(vm.new_type_error("Function annotations aren't a dict"))
    }
}

pub fn jit_ret_type(func: &Py<PyFunction>, vm: &VirtualMachine) -> PyResult<Option<JitType>> {
    let func_obj: PyObjectRef = func.as_ref().to_owned();
    let annotations = func_obj.get_attr("__annotations__", vm)?;
    if vm.is_none(&annotations) {
        Err(new_jit_error(
            "Jitting function requires return type to have annotations".to_owned(),
            vm,
        ))
    } else if let Ok(dict) = PyDictRef::try_from_object(vm, annotations) {
        if dict.contains_key("return", vm) {
            get_jit_arg_type(&dict, "return", vm).map_or(Ok(None), |t| Ok(Some(t)))
        } else {
            Ok(None)
        }
    } else {
        Err(vm.new_type_error("Function annotations aren't a dict"))
    }
}
