#[unsafe(no_mangle)]
#[warn(dead_code)]

fn say_hello() {
    println!("Hello from Rust!");
}

#[unsafe(no_mangle)]
fn compute(
    x : std::ffi::c_double,
    y: std::ffi::c_double,
    c_operation: *const std::ffi::c_char,
) -> std::ffi::c_double {
    let operation = unsafe { std::ffi::CStr::from_ptr(c_operation) }.to_string_lossy();

    let result: std::ffi::c_double; 
    match operation.as_ref() {
        "add" => result = x + y,
        "subtract" => result = x - y,
        "multiply" => result = x * y,
        "divide" => result = x / y,
        _ => result = 0.0,
    };

    result
}

#[unsafe(no_mangle)]
fn transform(data:*mut std::ffi::c_double,len:usize,){
    let values = unsafe { std::slice::from_raw_parts_mut(data, len) };
    println!("{:?}", values.reverse());
    println!("{:?}", values);
}
