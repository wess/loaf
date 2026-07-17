/*
 * loaf.h — the stable C ABI exposed by libloaf (the `bun_embed` crate inside
 * the Bun fork). loaf-sys mirrors these declarations in Rust; the safe `loaf`
 * crate wraps them. Keep this file and sys/src/lib.rs in lockstep.
 *
 * Design notes live in docs/architecture.md. Summary of the contract:
 *   - A LoafRuntime owns a JavaScriptCore VM + global object + event loop and
 *     is single-threaded: never touch one runtime from two threads.
 *   - A LoafValue is an opaque, GC-protected handle. The caller owns every
 *     handle a function returns and must release it with loaf_value_free
 *     (handles are also all released when the runtime is freed).
 *   - Fallible calls return LoafStatus and write results / the pending JS
 *     exception into out-parameters. Nothing unwinds across this boundary.
 */
#ifndef LOAF_H
#define LOAF_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define LOAF_ABI_VERSION 1u

typedef struct LoafRuntime LoafRuntime;
typedef struct LoafValue LoafValue;

typedef enum LoafStatus {
  LOAF_OK = 0,
  LOAF_ERR_EXCEPTION = 1, /* a JS exception was thrown; see the out_err param */
  LOAF_ERR_SYNTAX = 2,    /* source failed to parse or transpile */
  LOAF_ERR_TYPE = 3,      /* type error (e.g. calling a non-function) */
  LOAF_ERR_UTF8 = 4,      /* invalid UTF-8 crossed the boundary */
  LOAF_ERR_INTERNAL = 5   /* internal / runtime failure */
} LoafStatus;

typedef enum LoafType {
  LOAF_TYPE_UNDEFINED = 0,
  LOAF_TYPE_NULL = 1,
  LOAF_TYPE_BOOLEAN = 2,
  LOAF_TYPE_NUMBER = 3,
  LOAF_TYPE_STRING = 4,
  LOAF_TYPE_SYMBOL = 5,
  LOAF_TYPE_OBJECT = 6,
  LOAF_TYPE_ARRAY = 7,
  LOAF_TYPE_FUNCTION = 8,
  LOAF_TYPE_PROMISE = 9,
  LOAF_TYPE_BIGINT = 10,
  LOAF_TYPE_OTHER = 11
} LoafType;

typedef enum LoafLang {
  LOAF_LANG_JS = 0,
  LOAF_LANG_TS = 1,
  LOAF_LANG_JSX = 2,
  LOAF_LANG_TSX = 3
} LoafLang;

typedef enum LoafModuleKind {
  LOAF_SCRIPT = 0, /* classic global script */
  LOAF_MODULE = 1  /* ES module */
} LoafModuleKind;

typedef struct LoafRuntimeOptions {
  int32_t install_bun_globals; /* install Bun.*, fetch, etc. */
  int32_t install_console;     /* install console.* */
  size_t heap_size_hint;       /* bytes; 0 = engine default */
} LoafRuntimeOptions;

typedef struct LoafEvalOptions {
  LoafLang lang;
  LoafModuleKind module;
  const char *filename; /* for stack traces / resolution; may be NULL */
  size_t filename_len;
} LoafEvalOptions;

/* A UTF-8 buffer owned by libloaf; release with loaf_bytes_free. */
typedef struct LoafBytes {
  const uint8_t *ptr;
  size_t len;
} LoafBytes;

/*
 * Host callback invoked when JS calls a function made by loaf_function_new.
 * On success return LOAF_OK and write an owned handle to *out_ret (NULL means
 * undefined). To throw, return LOAF_ERR_EXCEPTION and write the value to throw
 * to *out_ret. `this_val` and the `argv` handles are borrowed — do not free
 * them; loaf_value_dup if you need to keep one.
 */
typedef LoafStatus (*LoafHostFn)(LoafRuntime *rt, void *userdata,
                                 LoafValue *this_val,
                                 LoafValue *const *argv, size_t argc,
                                 LoafValue **out_ret);

/* Runs when the JS function is garbage-collected, to release `userdata`. */
typedef void (*LoafFinalizer)(void *userdata);

/* ---- lifecycle ---------------------------------------------------------- */
uint32_t loaf_abi_version(void);
LoafRuntime *loaf_runtime_new(const LoafRuntimeOptions *opts);
void loaf_runtime_free(LoafRuntime *rt);
LoafValue *loaf_globals(LoafRuntime *rt);

/* ---- value lifetime ----------------------------------------------------- */
void loaf_value_free(LoafValue *v);
LoafValue *loaf_value_dup(LoafRuntime *rt, LoafValue *v);
LoafType loaf_value_type(LoafRuntime *rt, LoafValue *v);
int32_t loaf_value_strict_equals(LoafRuntime *rt, LoafValue *a, LoafValue *b);

/* ---- primitive constructors --------------------------------------------- */
LoafValue *loaf_undefined(LoafRuntime *rt);
LoafValue *loaf_null(LoafRuntime *rt);
LoafValue *loaf_boolean(LoafRuntime *rt, int32_t b);
LoafValue *loaf_number(LoafRuntime *rt, double n);
LoafValue *loaf_string(LoafRuntime *rt, const char *utf8, size_t len);

/* ---- primitive accessors ------------------------------------------------ */
int32_t loaf_truthy(LoafRuntime *rt, LoafValue *v);
int32_t loaf_get_number(LoafRuntime *rt, LoafValue *v, double *out);
LoafStatus loaf_get_string(LoafRuntime *rt, LoafValue *v, LoafBytes *out);
void loaf_bytes_free(LoafBytes bytes);

/* ---- objects ------------------------------------------------------------ */
LoafValue *loaf_object_new(LoafRuntime *rt);
LoafStatus loaf_object_get(LoafRuntime *rt, LoafValue *obj, const char *key,
                           size_t klen, LoafValue **out);
LoafStatus loaf_object_set(LoafRuntime *rt, LoafValue *obj, const char *key,
                           size_t klen, LoafValue *val);
LoafStatus loaf_object_delete(LoafRuntime *rt, LoafValue *obj, const char *key,
                              size_t klen);
LoafStatus loaf_object_has(LoafRuntime *rt, LoafValue *obj, const char *key,
                           size_t klen, int32_t *out);
LoafStatus loaf_object_keys(LoafRuntime *rt, LoafValue *obj,
                            LoafValue **out_array);

/* ---- arrays ------------------------------------------------------------- */
LoafValue *loaf_array_new(LoafRuntime *rt);
LoafStatus loaf_array_length(LoafRuntime *rt, LoafValue *arr, uint32_t *out);
LoafStatus loaf_array_get(LoafRuntime *rt, LoafValue *arr, uint32_t i,
                          LoafValue **out);
LoafStatus loaf_array_set(LoafRuntime *rt, LoafValue *arr, uint32_t i,
                          LoafValue *val);
LoafStatus loaf_array_push(LoafRuntime *rt, LoafValue *arr, LoafValue *val);

/* ---- functions ---------------------------------------------------------- */
LoafValue *loaf_function_new(LoafRuntime *rt, LoafHostFn cb, void *userdata,
                             LoafFinalizer fin);
LoafStatus loaf_call(LoafRuntime *rt, LoafValue *fn, LoafValue *this_val,
                     LoafValue *const *argv, size_t argc, LoafValue **out_ret);
LoafStatus loaf_construct(LoafRuntime *rt, LoafValue *fn, LoafValue *const *argv,
                          size_t argc, LoafValue **out_ret);

/* ---- eval --------------------------------------------------------------- */
LoafStatus loaf_eval(LoafRuntime *rt, const char *src, size_t srclen,
                     const LoafEvalOptions *opts, LoafValue **out_ret);

/* ---- promises / event loop ---------------------------------------------- */
void loaf_run_event_loop(LoafRuntime *rt);
int32_t loaf_tick(LoafRuntime *rt);
int32_t loaf_is_promise(LoafRuntime *rt, LoafValue *v);
/* Fulfilled => LOAF_OK + *out_ret; rejected => LOAF_ERR_EXCEPTION and the
   rejection reason becomes the pending exception (loaf_take_pending_exception). */
LoafStatus loaf_await(LoafRuntime *rt, LoafValue *promise, LoafValue **out_ret);

/* ---- exceptions --------------------------------------------------------- */
/* Any call that returns LOAF_ERR_EXCEPTION leaves the thrown value pending on
   the runtime. Take ownership of it here (NULL if none is pending). */
LoafValue *loaf_take_pending_exception(LoafRuntime *rt);
/* Format a caught value (usually an Error) as "message\n stack" UTF-8. */
LoafStatus loaf_exception_message(LoafRuntime *rt, LoafValue *err,
                                  LoafBytes *out);

#ifdef __cplusplus
}
#endif
#endif /* LOAF_H */
