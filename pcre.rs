/* rustpcre - rust PCRE bindings
 *
 * Copyright 2011 Google Inc. All Rights Reserved.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *    http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

extern mod std;

use libc::{c_int, c_void, c_char};

#[allow(non_camel_case_types)]
type pcre_t = *c_void;

#[allow(non_camel_case_types)]
type pcre_extra_t = *c_void;

#[link_name = "pcre"]
extern mod libpcre {
    fn pcre_compile(pattern: *c_char, options: c_int, errptr: &*c_char,
                    erroffset: &c_int, tableptr: *u8) -> *pcre_t;
    fn pcre_exec(re: *pcre_t, extra: *pcre_extra_t, subject: *c_char,
                 length: c_int, startoffset: c_int, options: c_int,
                 ovector: *c_int, ovecsize: c_int) -> c_int;
    fn pcre_get_stringnumber(re: *pcre_t, name: *c_char) -> c_int;
    fn pcre_refcount(re: *pcre_t, adj: c_int) -> c_int;
}

pub struct Pcre {
    priv re: *pcre_t,

    drop {
        libpcre::pcre_refcount(self.re, -1 as c_int);
    }
}

fn Pcre(pattern: &str) -> Result<Pcre, ~str> unsafe {
    do str::as_c_str(pattern) |pattern| {
        let errv = ptr::null();
        let erroff = 0 as c_int;

        let re = libpcre::pcre_compile(
            pattern,
            0 as c_int,
            &errv,
            &erroff,
            ptr::null()
        );

        if re == ptr::null() {
            Err(str::raw::from_c_str(errv))
        } else {
            Ok(Pcre { re: re })
        }
    }
}

pub impl Pcre {
    fn exec(target: &str) -> Option<Match> unsafe {
        let oveclen = 30;
        let mut ovec = vec::from_elem(oveclen as uint, 0i32);
        let ovecp = vec::raw::to_ptr(ovec);

        let r = do str::as_c_str(target) |p| {
            libpcre::pcre_exec(
                self.re,
                ptr::null(),
                p,
                target.len() as c_int,
                0 as c_int,
                0 as c_int,
                ovecp,
                oveclen as c_int
            )
        };

        if r < 0i32 { return None; }

        let mut idx = 2;    // skip the whole-string match at the start
        let mut substrings = ~[];
        while idx < oveclen * 2 / 3 {
            let start = ovec[idx];
            let end = ovec[idx + 1];
            idx = idx + 2;
            if start != end && start >= 0i32 && end >= 0i32 {
                substrings.push(@target.slice(start as uint, end as uint));
            }
        }

        // Make sure we let pcre know that we're sharing the pcre_t pointer.
        libpcre::pcre_refcount(self.re, 1 as c_int);

        Some(Match {
            substrings: substrings,
            re: self.re
        })
    }
}

pub struct Match {
    priv re: *pcre_t,
    substrings: ~[@~str],

    drop {
        libpcre::pcre_refcount(self.re, -1 as c_int);
    }
}

pub impl Match {
    pure fn named(name: &str) -> Option<@~str> {
        let idx = do str::as_c_str(name) |name| {
            unsafe { libpcre::pcre_get_stringnumber(self.re, name) as int }
        };

        if idx > 0 {
            Some(self[idx as uint - 1])
        } else {
            None
        }
    }
}

pub impl Match: ops::Index<uint, @~str> {
    pure fn index(idx: uint) -> @~str {
        self.substrings[idx]
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_compile_fail() {
        assert result::unwrap_err(Pcre("(")) == ~"missing )";
    }

    #[test]
    fn test_match_basic() {
        let r = result::unwrap(Pcre("..."));
        let m = option::unwrap(r.exec("abc"));

        assert m.substrings.is_empty();
    }

    #[test]
    fn test_match_fail() {
        let r = result::unwrap(Pcre("...."));
        let m = r.exec("ab");

        assert m.is_none();
    }

    #[test]
    fn test_substring() {
        let r = result::unwrap(Pcre("(.)bcd(e.g)"));
        let m = option::unwrap(r.exec("abcdefg"));

        assert *m[0u] == ~"a";
        assert *m[1u] == ~"efg";
    }

    #[test]
    fn test_named() {
        let r = result::unwrap(Pcre("(?<foo>..).(?<bar>..)"));
        let m = option::unwrap(r.exec("abcde"));

        assert m.named("foo") == Some(@~"ab");
        assert m.named("bar") == Some(@~"de");
        assert m.named("baz") == None;
    }
}
