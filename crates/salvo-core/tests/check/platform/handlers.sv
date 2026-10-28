// [platform-handler] A `platform handler` is a handler of an ordinary
// effect whose implementation the *host* supplies in the target language,
// with a small set of restrictions, each because the implementation is not
// Salvo's.

// [platform-handler] The shape: an *ordinary* effect, a handler the host
// implements, registered with `use` like any handler. Nothing else about
// either declaration changes — which is the point of the form (FS-1
// resolved as O-M2, user decision 2026-09-14).
effect RawFs {
    fn raw_close(handle: Int) [] -> Bool => handle
}

platform handler HostRawFs of RawFs

fn shut_raw(handle: Int) [RawFs] -> Bool {
    return raw_close(handle)
}

fn registers() [use] -> None {
    use HostRawFs()
    shut_raw(1)
}

// [platform-handler] [effect-handle] [platform-threadsafe] A platform
// handler binds as a handle either way (user decision 2026-09-26): without
// `threadsafe` as a monitor (serialized behind a lock on both backends),
// with it bare (shared raw). Either satisfies a `[RawFs]`, so a handler
// depending on its effect — the `DefaultFs [RawFs] of Fs` shape — binds whichever
// the host declared. The two kinds are told apart in the backends' emission
// tests.
effect Fs {
    fn shut(handle: Int) -> Bool => handle
}

handler DefaultFs [RawFs] of Fs {
    fn shut(handle: Int) -> Bool => handle {
        return raw_close(handle)
    }
}

fn close_it(handle: Int) [Fs] -> Bool {
    return shut(handle)
}

fn shares() [use] -> None {
    use HostRawFs()
    use DefaultFs()
    close_it(1)
}

// [platform-threadsafe] The declared form checks identically: `threadsafe`
// changes how the instance is shared, never whether it may be.
threadsafe platform handler SharedRawFs of RawFs

fn shares_threadsafe() [use] -> None {
    use SharedRawFs()
    use DefaultFs()
    close_it(1)
}

// [platform-handler] Constructor parameters are the host class's, passed
// through by the `use` site: `use HostS3("bucket")` is how a host
// implementation is configured.
effect Store {
    fn put(key: Str) [] -> None => key
}

platform handler HostS3(bucket: Str) of Store

fn configured() [use] -> None {
    use HostS3("salvo")
    put("k")
}

// [platform-handler] [platform-intrinsic-std-only] Unlike `intrinsic`, `platform` is
// the *customer's* modifier: the implementation is a file in their source
// tree, not a table inside the compiler, so a program may declare one.
effect Clock {
    fn now() [] -> Int
}

platform handler HostClock of Clock

// [platform-handler] The members are the host's, in the target language: a
// member written with a Salvo body is refused, naming the implementation
// file.
platform handler BodyClock of Clock {
    //~ error[platform-handler:body]
    fn now() -> Int {
        return 0
    }
}

// [platform-handler] A platform handler declares parameters, not state:
// its implementation class owns its fields (user decision 2026-10-01, the ABI decisions
// D8).
platform handler StateClock of Clock {
    state {
        //~ error[platform-handler:state]
        ticks: Int = 0
    }
}

// [platform-handler] [effect-handler-deps] A dependency is supplied *to a
// handler's members*, and these members are host code, which performs no
// Salvo effect. The remedy is a Salvo handler in between — which is exactly
// what phase 4's `DefaultFs [RawFs] of Fs` is.
effect Logger {
    fn log(message: Str) -> None => message
}

//~ error[platform-handler:dependencies]
platform handler LoggedClock [Logger] of Clock

// [ref-handle] A `ref` result is a path into the caller's container, which
// host code does not answer (user decision 2026-10-10): a platform handler
// cannot implement a member returning one, restated or not.
struct Entity canbe Mut { hp: Int }

effect Lender {
    fn lease(es: List<Mut Entity>) -> (ref(es) Mut Entity)? => es
}

//~ error[ref-handle:platform-handler]
platform handler HostLender of Lender

// [platform-handler] Generic-free: the host writes one concrete class, and a `use` site has no instance per
// type argument to construct [backend-never-wrong].
effect Bin<T> {
    fn keep(value: T) [] -> None => !value
}

//~ error[platform-handler:generic]
platform handler HostBin<T> of Bin<T>
