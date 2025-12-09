#include <cstdarg>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>



extern "C" {

/// # Safety
/// 提供给 C 侧释放字符串指针，调用者需要保证指针来自 Rust
void free_str(char *s);

const char *hello(const char *name);

/// # Safety
/// 这个函数是不安全的，别调！
const char *hello_bad(const char *name);

const char *hello_not_safe(const char *name);

const char *hello_world();

}  // extern "C"
