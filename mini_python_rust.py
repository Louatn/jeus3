import sys
import ctypes

print()
native_lib_name=sys.argv[1]
print('• loading', native_lib_name)
native_lib=ctypes.CDLL(native_lib_name)
print(native_lib['compute'])

print()
function_1_name='say_hello'
print('• accessing', function_1_name)
function_1=native_lib[function_1_name]
print('• calling', function_1_name)
function_1()


function_2_name='compute'
print('• accessing', function_2_name)
function_2=native_lib[function_2_name]
print('• calling', function_2_name)
function_2.argtypes=[ctypes.c_double, ctypes.c_double, ctypes.c_char_p]
function_2.restype=ctypes.c_double
print(function_2(3.0, 4.0, 'add'.encode()))
print(function_2(3.0, 4.0, 'subtract'.encode()))
print(function_2(3.0, 4.0, 'multiply'.encode())) 
print(function_2(3.0, 4.0, 'divide'.encode()))


function_3_name='transform'
print('• accessing', function_3_name)
function_3=native_lib[function_3_name]
print('• calling', function_3_name)
function_3.argtypes=[ctypes.c_void_p, ctypes.c_size_t]
function_3.restype=None
array_type=ctypes.c_double * 5
function_3(array_type(1.0, 2.0, 3.0, 4.0, 5.0), 5)