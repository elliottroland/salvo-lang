// what the host hands back: checked at the boundary, built canonically
//
// Compile-and-run data ([test-run-data]), moved from the CLI's platform
// tests. Each program's platform files are the skeletons `salvo platform
// generate` writes, their stubs implemented.

// [platform-check] The adapters and wrappers check what the host hands
// back (ABI D7, D10 C3): a closed literal union, a literal field of a
// struct inside a list (element by element), a state qualifier
// (`NonEmpty`, by running its `qualifies`), a literal arm of a positional
// union, and a `Reply<T>` the host completes from its own thread — the
// hosts building their unions with the factories [platform-factory] (a
// named union's `Problems.notFound` / `Problem::not_found`, a signature's
// `Open.err`, a literal arm's checked `Find.str`). Good values pass; the
// two programs after this one hand back a bad one.
//--- the_boundary_passes_what_the_host_returns
// FILE: salvo.toml
[project]
name = "case"

[build]
platform = "platform"
// FILE: platform/main.rs
use crate::*;
use crate::unions::*;

pub fn tier(n: i32) -> Option<String> {
    match n { 1 => Some("gold".to_string()), 2 => None, _ => Some("bronze".to_string()) }
}

pub fn items(n: i32) -> Vec<crate::Item> {
    vec![Item { tier: "gold".to_string(), tags: vec!["a".to_string(), "b".to_string()] }]
}

pub fn names(n: i32) -> Vec<String> {
    if n > 0 { vec!["x".to_string()] } else { vec![] }
}

pub fn open(p: &String) -> crate::unions::Union2<String, crate::unions::Union2<crate::NotFound, crate::Denied>> {
    Open::err(Problem::not_found(NotFound { path: p.clone() }))
}

pub struct HostLookup {
}

impl HostLookup {
    pub fn new() -> Self {
        Self { }
    }
}

impl crate::LookupPlatformSync for HostLookup {
    fn find(&self, key: &String) -> crate::unions::Union2<i32, String> {
        Find::str("missing".to_string())
    }
    fn parity(&self, n: i32, done: crate::scheduler::SalvoReply) {
        let done = done.hosted();
        std::thread::spawn(move || {
            let v = if n < 0 { "neither" } else if n % 2 == 0 { "even" } else { "odd" };
            done.send(v.to_string())
        });
    }
}
// FILE: platform/main.kt
package salvo.platform.main

import salvo.*
import salvo.main.*
fun tier(n: Int): String? {
    return when (n) { 1 -> "gold"; 2 -> null; else -> "bronze" }
}
fun items(n: Int): List<Item> {
    return listOf(Item(tier = "gold", tags = listOf("a", "b")))
}
fun names(n: Int): List<String> {
    return if (n > 0) listOf("x") else listOf()
}
fun open(p: String): Union2<String, Union2<NotFound, Denied>> {
    return Open.err(Problems.notFound(NotFound(p)))
}

class HostLookup : LookupPlatform {
    override fun find(key: String): Union2<Int, String> {
        return Find.str("missing")
    }
    override fun parity(n: Int, done: salvo.SalvoReply) {
        val host = done.hosted()
        Thread {
            val v = if (n < 0) "neither" else if (n % 2 == 0) "even" else "odd"
            try { host.send(v) } catch (e: IllegalStateException) { System.err.println(e.message); System.exit(3) }
        }.start()
    }
}
// FILE: main.sv
struct Item {
    tier: "gold" | "silver",
    tags: List<"a" | "b">
}

platform fn tier(n: Int) [] -> "gold" | "silver" | None

platform fn items(n: Int) [] -> List<Item>

platform fn names(n: Int) [] -> NonEmpty List<Str>

struct NotFound { path: Str }
struct Denied { path: Str }
type Problem = NotFound | Denied

platform fn open(p: Str) [] -> Ok Str | Err Problem => p

effect Lookup {
    fn find(key: Str) [] -> Int | "missing" => key
    fn parity(n: Int, done: Reply<"even" | "odd">) [] -> None => !n, !done
}

threadsafe platform handler HostLookup() of Lookup

fn show(t: "gold" | "silver" | None) -> Str {
    when t {
        is None { return "none" }
        is "gold" { return "gold" }
        is "silver" { return "silver" }
    }
}

fn main() [use] {
    use StdConsole()
    use HostLookup()
    println("${show(tier(1))} ${show(tier(2))}")
    println("${items(1).size()} ${names(1)}")
    let o = open("x")
    when o {
        is Ok { println("ok") }
        is ^Err {
            when o {
                is NotFound { println("not found ${o.path}") }
                is Denied { println("denied") }
            }
        }
    }
    let found = find("x")
    when found {
        is Int { println("int") }
        is "missing" { println("missing") }
    }
    let p = waitfor done: Reply<"even" | "odd"> { parity(3, done) }
    println("${p}")
    println(show(tier(1)))
}
//= stdout
//= gold none
//= 1 [x]
//= not found x
//= missing
//= odd
//= gold

// [platform-check] A bad result fails where it crosses — Rust panics,
// Kotlin throws — naming the declaration and the value: here `tier(3)`
// answers "bronze", which `"gold" | "silver" | None` does not hold.
//--- a_bad_platform_fn_result_traps
// FILE: salvo.toml
[project]
name = "case"

[build]
platform = "platform"
// FILE: platform/main.rs
use crate::*;
use crate::unions::*;

pub fn tier(n: i32) -> Option<String> {
    match n { 1 => Some("gold".to_string()), 2 => None, _ => Some("bronze".to_string()) }
}

pub fn items(n: i32) -> Vec<crate::Item> {
    vec![Item { tier: "gold".to_string(), tags: vec!["a".to_string(), "b".to_string()] }]
}

pub fn names(n: i32) -> Vec<String> {
    if n > 0 { vec!["x".to_string()] } else { vec![] }
}

pub fn open(p: &String) -> crate::unions::Union2<String, crate::unions::Union2<crate::NotFound, crate::Denied>> {
    Open::err(Problem::not_found(NotFound { path: p.clone() }))
}

pub struct HostLookup {
}

impl HostLookup {
    pub fn new() -> Self {
        Self { }
    }
}

impl crate::LookupPlatformSync for HostLookup {
    fn find(&self, key: &String) -> crate::unions::Union2<i32, String> {
        Find::str("missing".to_string())
    }
    fn parity(&self, n: i32, done: crate::scheduler::SalvoReply) {
        let done = done.hosted();
        std::thread::spawn(move || {
            let v = if n < 0 { "neither" } else if n % 2 == 0 { "even" } else { "odd" };
            done.send(v.to_string())
        });
    }
}
// FILE: platform/main.kt
package salvo.platform.main

import salvo.*
import salvo.main.*
fun tier(n: Int): String? {
    return when (n) { 1 -> "gold"; 2 -> null; else -> "bronze" }
}
fun items(n: Int): List<Item> {
    return listOf(Item(tier = "gold", tags = listOf("a", "b")))
}
fun names(n: Int): List<String> {
    return if (n > 0) listOf("x") else listOf()
}
fun open(p: String): Union2<String, Union2<NotFound, Denied>> {
    return Open.err(Problems.notFound(NotFound(p)))
}

class HostLookup : LookupPlatform {
    override fun find(key: String): Union2<Int, String> {
        return Find.str("missing")
    }
    override fun parity(n: Int, done: salvo.SalvoReply) {
        val host = done.hosted()
        Thread {
            val v = if (n < 0) "neither" else if (n % 2 == 0) "even" else "odd"
            try { host.send(v) } catch (e: IllegalStateException) { System.err.println(e.message); System.exit(3) }
        }.start()
    }
}
// FILE: main.sv
struct Item {
    tier: "gold" | "silver",
    tags: List<"a" | "b">
}

platform fn tier(n: Int) [] -> "gold" | "silver" | None

platform fn items(n: Int) [] -> List<Item>

platform fn names(n: Int) [] -> NonEmpty List<Str>

struct NotFound { path: Str }
struct Denied { path: Str }
type Problem = NotFound | Denied

platform fn open(p: Str) [] -> Ok Str | Err Problem => p

effect Lookup {
    fn find(key: Str) [] -> Int | "missing" => key
    fn parity(n: Int, done: Reply<"even" | "odd">) [] -> None => !n, !done
}

threadsafe platform handler HostLookup() of Lookup

fn show(t: "gold" | "silver" | None) -> Str {
    when t {
        is None { return "none" }
        is "gold" { return "gold" }
        is "silver" { return "silver" }
    }
}

fn main() [use] {
    use StdConsole()
    use HostLookup()
    println("${show(tier(1))} ${show(tier(2))}")
    println("${items(1).size()} ${names(1)}")
    let o = open("x")
    when o {
        is Ok { println("ok") }
        is ^Err {
            when o {
                is NotFound { println("not found ${o.path}") }
                is Denied { println("denied") }
            }
        }
    }
    let found = find("x")
    when found {
        is Int { println("int") }
        is "missing" { println("missing") }
    }
    let p = waitfor done: Reply<"even" | "odd"> { parity(3, done) }
    println("${p}")
    println(show(tier(3)))
}
// The value is quoted on both backends (on Kotlin since 2026-10-28).
//= trap `platform fn tier`'s result was "bronze", which is not one of "gold", "silver" [platform-check]
//= stdout
//= gold none
//= 1 [x]
//= not found x
//= missing
//= odd

// [platform-check] And a bad value the host sends on a `Reply` from its own
// thread: `parity(-1)` sends "neither".
//--- a_bad_hosted_reply_traps
// FILE: salvo.toml
[project]
name = "case"

[build]
platform = "platform"
// FILE: platform/main.rs
use crate::*;
use crate::unions::*;

pub fn tier(n: i32) -> Option<String> {
    match n { 1 => Some("gold".to_string()), 2 => None, _ => Some("bronze".to_string()) }
}

pub fn items(n: i32) -> Vec<crate::Item> {
    vec![Item { tier: "gold".to_string(), tags: vec!["a".to_string(), "b".to_string()] }]
}

pub fn names(n: i32) -> Vec<String> {
    if n > 0 { vec!["x".to_string()] } else { vec![] }
}

pub fn open(p: &String) -> crate::unions::Union2<String, crate::unions::Union2<crate::NotFound, crate::Denied>> {
    Open::err(Problem::not_found(NotFound { path: p.clone() }))
}

pub struct HostLookup {
}

impl HostLookup {
    pub fn new() -> Self {
        Self { }
    }
}

impl crate::LookupPlatformSync for HostLookup {
    fn find(&self, key: &String) -> crate::unions::Union2<i32, String> {
        Find::str("missing".to_string())
    }
    fn parity(&self, n: i32, done: crate::scheduler::SalvoReply) {
        let done = done.hosted();
        std::thread::spawn(move || {
            let v = if n < 0 { "neither" } else if n % 2 == 0 { "even" } else { "odd" };
            done.send(v.to_string())
        });
    }
}
// FILE: platform/main.kt
package salvo.platform.main

import salvo.*
import salvo.main.*
fun tier(n: Int): String? {
    return when (n) { 1 -> "gold"; 2 -> null; else -> "bronze" }
}
fun items(n: Int): List<Item> {
    return listOf(Item(tier = "gold", tags = listOf("a", "b")))
}
fun names(n: Int): List<String> {
    return if (n > 0) listOf("x") else listOf()
}
fun open(p: String): Union2<String, Union2<NotFound, Denied>> {
    return Open.err(Problems.notFound(NotFound(p)))
}

class HostLookup : LookupPlatform {
    override fun find(key: String): Union2<Int, String> {
        return Find.str("missing")
    }
    override fun parity(n: Int, done: salvo.SalvoReply) {
        val host = done.hosted()
        Thread {
            val v = if (n < 0) "neither" else if (n % 2 == 0) "even" else "odd"
            try { host.send(v) } catch (e: IllegalStateException) { System.err.println(e.message); System.exit(3) }
        }.start()
    }
}
// FILE: main.sv
struct Item {
    tier: "gold" | "silver",
    tags: List<"a" | "b">
}

platform fn tier(n: Int) [] -> "gold" | "silver" | None

platform fn items(n: Int) [] -> List<Item>

platform fn names(n: Int) [] -> NonEmpty List<Str>

struct NotFound { path: Str }
struct Denied { path: Str }
type Problem = NotFound | Denied

platform fn open(p: Str) [] -> Ok Str | Err Problem => p

effect Lookup {
    fn find(key: Str) [] -> Int | "missing" => key
    fn parity(n: Int, done: Reply<"even" | "odd">) [] -> None => !n, !done
}

threadsafe platform handler HostLookup() of Lookup

fn show(t: "gold" | "silver" | None) -> Str {
    when t {
        is None { return "none" }
        is "gold" { return "gold" }
        is "silver" { return "silver" }
    }
}

fn main() [use] {
    use StdConsole()
    use HostLookup()
    println("${show(tier(1))} ${show(tier(2))}")
    println("${items(1).size()} ${names(1)}")
    let o = open("x")
    when o {
        is Ok { println("ok") }
        is ^Err {
            when o {
                is NotFound { println("not found ${o.path}") }
                is Denied { println("denied") }
            }
        }
    }
    let found = find("x")
    when found {
        is Int { println("int") }
        is "missing" { println("missing") }
    }
    let p = waitfor done: Reply<"even" | "odd"> { parity(-1, done) }
    println("${p}")
    println(show(tier(1)))
}
//= trap what the host sent on `Lookup.parity` was "neither", which is not one of "even", "odd" [platform-check]
//= stdout
//= gold none
//= 1 [x]
//= not found x
//= missing

// [platform-value-type] A set or map the host returns is std's own host
// type (ROADMAP §0j step 7), built with the canonical builders the host
// files offer (`canonicalSet`, `canonical_map`, …): Salvo then finds the
// elements by its own `hash`/`eq`/`cmp`, and a sorted set of strings is in
// code-point order on both backends (UTF-16 order would put the emoji
// first).
//--- sets_and_maps_from_the_host_are_built_canonically
// FILE: salvo.toml
[project]
name = "case"

[build]
platform = "platform"
// FILE: platform/main.rs
use crate::*;

pub fn names() -> crate::Set<String> {
    crate::platform_std_core_set::canonical_set(vec!["b".to_string(), "a".to_string(), "c".to_string()])
}

pub fn sorted() -> crate::SortedSet<String> {
    crate::platform_std_core_sorted::canonical_sorted_set(vec!["\u{FF61}".to_string(), "\u{1F600}".to_string()])
}

pub fn ages() -> crate::Map<String, i32> {
    crate::platform_std_core_map::canonical_map(vec![("x".to_string(), 1), ("y".to_string(), 2)])
}

pub fn bag() -> crate::Bag {
    Bag { tags: crate::platform_std_core_set::canonical_set(vec!["a".to_string(), "b".to_string()]) }
}
// FILE: platform/main.kt
package salvo.platform.main

import salvo.main.*
fun names(): salvo.platform.std.core.set.Set<String> {
    return salvo.platform.std.core.set.canonicalSet(listOf("b", "a", "c"))
}
fun sorted(): salvo.platform.std.core.sorted.SortedSet<String> {
    return salvo.platform.std.core.sorted.canonicalSortedSet(listOf("\uFF61", "\uD83D\uDE00"))
}
fun ages(): salvo.platform.std.core.map.Map<String, Int> {
    return salvo.platform.std.core.map.canonicalMap(listOf("x" to 1, "y" to 2))
}
fun bag(): Bag {
    return Bag(tags = salvo.platform.std.core.set.canonicalSet(listOf("a", "b")))
}
// FILE: main.sv
struct Bag {
    tags: Set<Str>
}

platform fn names() [] -> Set<Str>

platform fn sorted() [] -> SortedSet<Str>

platform fn ages() [] -> Map<Str, Int>

platform fn bag() [] -> Bag

fn main() [use] {
    use StdConsole()
    let ns = names()
    let ag = ages()
    let has = contains(ns, "a")
    let y = get(ag, "y") ?: unreachable!()
    println("${size(ns)} ${has} ${sorted()} ${y} ${size(bag().tags)}")
}
//= stdout
//= 3 true {｡, 😀} 2 2
