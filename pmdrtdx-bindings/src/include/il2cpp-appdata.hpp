// Generated C++ file by Il2CppInspector - http://www.djkaty.com - https://github.com/djkaty
// IL2CPP application data

#pragma once

// The compiler on macOS complains without this
#define __arm64__

#include <stdint.h>
#include <stddef.h>

// Application-specific types
#include "il2cpp-types.hpp"

// IL2CPP API function pointers
#include "il2cpp-api-functions-ptr.hpp"

// IL2CPP APIs
#include "il2cpp-api-functions.hpp"

// Application-specific functions
#define DO_APP_FUNC(a, r, n, p) extern "C" r n p

#define DO_APP_FUNC_METHODINFO(a, n) extern "C" struct MethodInfo* n

#include "il2cpp-functions.hpp"
#undef DO_APP_FUNC
#undef DO_APP_FUNC_METHODINFO

// TypeInfo pointers
#define DO_TYPEDEF(a, n) extern "C" n ## __Class* n ## __TypeInfo
#include "il2cpp-types-ptr.hpp"
#undef DO_TYPEDEF
