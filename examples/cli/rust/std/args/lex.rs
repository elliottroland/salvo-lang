use crate::std_core_list::add_platform;
use crate::std_core_string::char_at_platform;
use crate::std_core_string::split_once;
use crate::std_core_string::starts_with_platform;
use crate::std_core_string::trim_prefix_platform;


#[derive(Clone, Debug, PartialEq)]
pub struct LongToken {
    pub name: String,
    pub value: Option<String>,
}

impl crate::wire::__Wire for LongToken {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.name, out);
        crate::wire::__Wire::__enc(&self.value, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            name: crate::wire::__Wire::__dec(r)?,
            value: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShortToken {
    pub letters: String,
}

impl crate::wire::__Wire for ShortToken {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.letters, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            letters: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WordToken {
    pub value: String,
}

impl crate::wire::__Wire for WordToken {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.value, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            value: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EndToken {
}

impl crate::wire::__Wire for EndToken {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

pub fn lex(args: &Vec<String>) -> Vec<crate::unions::Union4<crate::std_args_lex::LongToken, crate::std_args_lex::ShortToken, crate::std_args_lex::WordToken, crate::std_args_lex::EndToken>> {
    let mut out: Vec<crate::unions::Union4<crate::std_args_lex::LongToken, crate::std_args_lex::ShortToken, crate::std_args_lex::WordToken, crate::std_args_lex::EndToken>> = vec![];
    let mut ended: bool = false;
    for mut arg in args.iter() {
        let mut a: String = (arg).clone();
        if ended {
            crate::std_core_list::add_platform::<crate::unions::Union4<crate::std_args_lex::LongToken, crate::std_args_lex::ShortToken, crate::std_args_lex::WordToken, crate::std_args_lex::EndToken>>(&mut out, crate::unions::Union4::U3(crate::std_args_lex::WordToken { value: a }));
        } else if (&a[..] == &String::from("--")[..]) {
            ended = true;
            crate::std_core_list::add_platform::<crate::unions::Union4<crate::std_args_lex::LongToken, crate::std_args_lex::ShortToken, crate::std_args_lex::WordToken, crate::std_args_lex::EndToken>>(&mut out, crate::unions::Union4::U4(crate::std_args_lex::EndToken {}));
        } else if crate::std_core_string::starts_with_platform(&a, &String::from("--")) {
            let mut body: String = crate::std_core_string::trim_prefix_platform(&a, &String::from("--"));
            let mut parts: Option<(String, String)> = crate::std_core_string::split_once(&body, &String::from("="));
            if parts.is_none() {
                crate::std_core_list::add_platform::<crate::unions::Union4<crate::std_args_lex::LongToken, crate::std_args_lex::ShortToken, crate::std_args_lex::WordToken, crate::std_args_lex::EndToken>>(&mut out, crate::unions::Union4::U1(crate::std_args_lex::LongToken { name: body, value: None }));
            } else {
                let mut parts_1 = parts.unwrap();
                let mut __destructured_2: (String, String) = parts_1;
                let mut name: String = __destructured_2.0;
                let mut value: String = __destructured_2.1;
                crate::std_core_list::add_platform::<crate::unions::Union4<crate::std_args_lex::LongToken, crate::std_args_lex::ShortToken, crate::std_args_lex::WordToken, crate::std_args_lex::EndToken>>(&mut out, crate::unions::Union4::U1(crate::std_args_lex::LongToken { name: name, value: Some(value) }));
            };
        } else if ((crate::std_core_string::starts_with_platform(&a, &String::from("-")) && !((&a[..] == &String::from("-")[..]))) && !(crate::std_args_lex::is_number(&a))) {
            crate::std_core_list::add_platform::<crate::unions::Union4<crate::std_args_lex::LongToken, crate::std_args_lex::ShortToken, crate::std_args_lex::WordToken, crate::std_args_lex::EndToken>>(&mut out, crate::unions::Union4::U2(crate::std_args_lex::ShortToken { letters: crate::std_core_string::trim_prefix_platform(&a, &String::from("-")) }));
        } else {
            crate::std_core_list::add_platform::<crate::unions::Union4<crate::std_args_lex::LongToken, crate::std_args_lex::ShortToken, crate::std_args_lex::WordToken, crate::std_args_lex::EndToken>>(&mut out, crate::unions::Union4::U3(crate::std_args_lex::WordToken { value: a }));
        };
    }
    return out;
}

pub fn is_number(a: &String) -> bool {
    let mut first: Option<char> = crate::std_core_string::char_at_platform(a, 1i32);
    if first.is_none() {
        return false;
    };
    let mut first_1 = first.unwrap();
    if crate::std_args_lex::is_digit(first_1) {
        return true;
    };
    let mut second: Option<char> = crate::std_core_string::char_at_platform(a, 2i32);
    let mut first_2 = first.unwrap();
    if ((first_2 == '.') && !(second.is_none())) {
        let mut second_3 = second.unwrap();
        return crate::std_args_lex::is_digit(second_3);
    };
    return false;
}

pub fn is_digit(mut c: char) -> bool {
    return ((c >= '0') && (c <= '9'));
}
