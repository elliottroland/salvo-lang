// [platform-intrinsic-std-only] `intrinsic` marks a declaration the *compiler*
// implements, so only the standard library may write it. A user source
// checked here is not std, so an `intrinsic` declaration in it is an
// error naming the one interop path customer code does have — a
// `platform handler`. (The declaration is fully explicit so this is the
// only diagnostic, not a decl-explicit complaint.)
//~ error[platform-intrinsic-std-only:outside-std]
intrinsic fn secret<T>(value: T) [] -> T => value
