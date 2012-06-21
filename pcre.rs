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

use std;

import libc::{c_int, c_void, c_char};
import dvec::{dvec, extensions};
import result::{result, ok, err, extensions};

export pcre, match;

type pcre_t = *c_void;
type pcre_extra_t = *c_void;

#[link_name = "pcre"]
native mod _native {
    fn pcre_compile(pattern: *c_char, options: c_int, errptr: **c_char,
                    erroffset: *c_int, tableptr: *u8) -> *pcre_t;
    fn pcre_exec(re: *pcre_t, extra: *pcre_extra_t, subject: *c_char,
                 length: c_int, startoffset: c_int, options: c_int,
                 ovector: *c_int, ovecsize: c_int) -> c_int;
    fn pcre_get_stringnumber(re: *pcre_t, name: *u8) -> c_int;
    fn pcre_refcount(re: *pcre_t, adj: c_int) -> c_int;
}

class pcre_ {
    priv {
        let re: *pcre_t;
    }

    new(re: *pcre_t) {
        self.re = re;
    }

    drop {
        _native::pcre_refcount(self.re, -1 as c_int);
    }

    fn match(target: str) -> option<match> unsafe {
        let oveclen = 30;
        let ovec = vec::to_mut(vec::from_elem(oveclen as uint, 0i32));
        let ovecp = vec::unsafe::to_ptr(ovec);

        let r = str::as_c_str(target) { |_target|
            _native::pcre_exec(self.re, ptr::null(),
                               _target, str::len(target) as c_int,
                               0 as c_int, 0 as c_int, ovecp,
                               oveclen as c_int)
        };

        if r < 0i32 {
            ret none;
        }
        let mut idx = 2;    // skip the whole-string match at the start
        let mut res = dvec();
        while idx < oveclen * 2 / 3 {
            let start = ovec[idx];
            let end = ovec[idx + 1];
            idx = idx + 2;
            if start != end && start >= 0i32 && end >= 0i32 {
                res.push(@target.slice(start as uint, end as uint));
            }
        }

        some(match(@vec::from_mut(dvec::unwrap(res)), self.re))
    }
}

type pcre = pcre_;

fn pcre(pattern: str) -> result<pcre, @str> unsafe {
    let errv = ptr::null();
    let erroff = 0 as c_int;
    let re = str::as_c_str(pattern) { |pattern|
        _native::pcre_compile(pattern, 0 as c_int, ptr::addr_of(errv),
                              ptr::addr_of(erroff), ptr::null())
    };

    if re == ptr::null() {
        err(@str::unsafe::from_c_str(errv))
    } else {
        ok(pcre_(re))
    }
}

class match {
    priv {
        let re: *pcre_t;
    }

    let substrings: @[@str];

    new(substrings: @[@str], re: *pcre_t) {
        self.substrings = substrings;
        self.re = re;
    }

    fn [](index: uint) -> @str {
        self.substrings[index]
    }

    fn named(name: str) -> @str unsafe {
        let idx = str::as_buf(name){ |name|
            _native::pcre_get_stringnumber(self.re, name) as uint
        };
        self.substrings[idx - 1]
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_compile_fail() {
        alt pcre("(") {
          err(s) { assert *s == "missing )" }
          ok(r) { fail }
        }
    }

    #[test]
    fn test_match_basic() {
        let r = result::unwrap(pcre("..."));
        let m = r.match("abc").get();

        assert m.substrings.is_empty();
    }

    #[test]
    fn test_match_fail() {
        let r = result::unwrap(pcre("...."));
        let m = r.match("ab");

        assert m.is_none();
    }

    #[test]
    fn test_substring() {
        let r = result::unwrap(pcre("(.)bcd(e.g)"));
        let m = r.match("abcdefg").get();

        assert *m[0u] == "a";
        assert *m[1u] == "efg";
    }

    #[test]
    fn test_named() {
        let r = result::unwrap(pcre("(?<foo>..).(?<bar>..)"));
        let m = r.match("abcde").get();

        assert *m.named("foo") == "ab";
        assert *m.named("bar") == "de";
    }
}
