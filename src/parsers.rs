use nom;


#[derive(Debug, PartialEq)]
pub enum Hashline {
    OpenEnv(Environment),
    PlainLine(String),
}

#[derive(Debug, PartialEq)]
pub struct Environment {
    indent_depth: usize,
    name: String,
    opts: String,
    comment: String,
    is_list_like: bool,
}

impl Environment {
    pub fn latex_begin(&self) -> String {
        format!(r"{:ind$}\begin{{{}}}{}{:comment_sep$}{}",
                "",
                self.name,
                self.opts,
                "",
                self.comment,
                ind = self.indent_depth,
                comment_sep = if self.comment.is_empty() { 0 } else { 1 })
    }

    pub fn latex_end(&self) -> String {
        format!(r"{:ind$}\end{{{}}}", "", self.name, ind = self.indent_depth)
    }

    pub fn indent_depth(&self) -> usize {
        self.indent_depth
    }

    pub fn is_list_like(&self) -> bool {
        self.is_list_like
    }
}


// Hashline parsers
named!(
    list_env_parser<&[u8]>,
    ws!(alt!(tag!("itemize") | tag!("enumerate") | tag!("description")))
);
named!(escaped_colon<u8>, preceded!(specific_byte!('\\' as u8), specific_byte!(':' as u8)));
named!(escaped_percent<u8>, preceded!(specific_byte!('\\' as u8), specific_byte!('%' as u8)));
named!(name_parser<u8>, alt!(escaped_colon | none_of_bytes_as_bytes!(b":%([{ \t")));
named!(opts_parser<u8>, alt!(escaped_colon | escaped_percent | none_of_bytes_as_bytes!(b":%")));
named!(args_parser<u8>, alt!(escaped_percent | none_of_bytes_as_bytes!(b"%")));
named!(
    hashline_parser<Hashline>,
    do_parse!(
        ws: opt!(is_a!(" ")) >>
        tag!("# ") >>
        name: many1!(name_parser) >>
        opts: many0!(opts_parser) >>
        tag!(":") >>
        args: many0!(args_parser) >>
        comment: call!(nom::rest) >>
        (hashline_helper(ws.unwrap_or(&b""[..]), &name, &opts, &args, &comment))
    )
);
#[inline]
fn hashline_helper(ws: &[u8], name: &[u8], opts: &[u8], args: &[u8], comment: &[u8]) -> Hashline {
    use std::str::from_utf8;
    use self::Hashline::{PlainLine, OpenEnv};

    // It is ok to unwrap here, since we have checked for UTF-8 when we read the file
    let name_utf8 = from_utf8(name).unwrap().trim();
    let opts_utf8 = from_utf8(opts).unwrap().trim().replace("%", r"\%");
    let args_utf8 = from_utf8(args).unwrap().trim().replace("%", r"\%");
    let comment_utf8 = from_utf8(comment).unwrap().trim();

    if args_utf8.is_empty() {
        // If no args are given, it's an environment
        let env = Environment {
            indent_depth: ws.len(),
            name: name_utf8.to_string(),
            opts: opts_utf8.to_string(),
            comment: comment_utf8.to_string(),
            is_list_like: list_env_parser(name).is_done(),
        };
        OpenEnv(env)
    } else {
        // If there are some args, it's a single-line command
        let ws_utf8 = from_utf8(ws).unwrap();
        PlainLine(format!(r"{}\{}{}{{{}}}{:comment_sep$}{}",
                          ws_utf8,
                          name_utf8,
                          opts_utf8,
                          args_utf8,
                          "",
                          comment_utf8,
                          comment_sep = if comment_utf8.is_empty() { 0 } else { 1 }))
    }
}

// Hashline processing
#[inline]
fn process_hashline<T: AsRef<str>>(line: T) -> Option<Hashline> {
    use nom::IResult::{Done, Error, Incomplete};

    match hashline_parser(line.as_ref().as_bytes()) {
        Done(_, r) => Some(r),
        Error(_) | Incomplete(_) => None,
    }
}


// Itemline parsers
named!(
    itemline_parser<Hashline>,
    do_parse!(
        ws: opt!(is_a!(" ")) >>
        tag!("*") >>
        item: call!(nom::rest) >>
        (itemline_helper(ws.unwrap_or(&b""[..]), item))
    )
);
#[inline]
fn itemline_helper(ws: &[u8], item: &[u8]) -> Hashline {
    use std::str::from_utf8;
    use self::Hashline::PlainLine;

    let ws_utf8 = from_utf8(ws).unwrap();
    let item_utf8 = from_utf8(item).unwrap().trim();

    PlainLine(format!(r"{}\item{:item_sep$}{}",
                      ws_utf8,
                      "",
                      item_utf8,
                      item_sep = if item_utf8.is_empty() { 0 } else { 1 }))
}

// Itemline processing
#[inline]
fn process_itemline<T: AsRef<str>>(line: T) -> Option<Hashline> {
    use nom::IResult::{Done, Error, Incomplete};

    match itemline_parser(line.as_ref().as_bytes()) {
        Done(_, r) => Some(r),
        Error(_) | Incomplete(_) => None,
    }
}

// Fully process line
pub fn process_line<T>(line: T, list_like_active: bool) -> Hashline
    where T: AsRef<str>
{
    use self::Hashline::PlainLine;

    match process_hashline(&line) {
        Some(r) => r,
        None => {
            if list_like_active {
                match process_itemline(&line) {
                    Some(r) => r,
                    None => PlainLine(line.as_ref().to_string()),
                }
            } else {
                PlainLine(line.as_ref().to_string())
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use nom::IResult::{Done, Error, Incomplete};
    use nom::{ErrorKind, Needed};

    #[test]
    fn itemline_helper() {
        use super::{Hashline, itemline_helper};

        assert_eq!(itemline_helper(&b"  "[..], &b"foo"[..]),
                   Hashline::PlainLine("  \\item foo".to_string()));
        // Test that no whitespace is put after `\item` if no item is given
        assert_eq!(itemline_helper(&b" "[..], &b""[..]),
                   Hashline::PlainLine(" \\item".to_string()));
    }

    #[test]
    fn process_itemline() {
        use super::{Hashline, process_itemline};

        // Valid itemlines
        assert_eq!(process_itemline("*"),
                   Some(Hashline::PlainLine("\\item".to_string())));
        assert_eq!(process_itemline("*  "),
                   Some(Hashline::PlainLine("\\item".to_string())));
        assert_eq!(process_itemline("  *"),
                   Some(Hashline::PlainLine("  \\item".to_string())));
        assert_eq!(process_itemline("  *  "),
                   Some(Hashline::PlainLine("  \\item".to_string())));
        assert_eq!(process_itemline("* foo"),
                   Some(Hashline::PlainLine("\\item foo".to_string())));
        assert_eq!(process_itemline("  * bar"),
                   Some(Hashline::PlainLine("  \\item bar".to_string())));
        assert_eq!(process_itemline("****"),
                   Some(Hashline::PlainLine("\\item ***".to_string())));

        // Not an itemline
        assert_eq!(process_itemline("  baz"), None);
        assert_eq!(process_itemline("qux *"), None);
        assert_eq!(process_itemline("  abc * def"), None);
        assert_eq!(process_itemline("  \\*  "), None);
        assert_eq!(process_itemline("\\*  "), None);
    }

    #[test]
    fn environment_methods() {
        use super::Environment;

        let env_1 = Environment {
            indent_depth: 0,
            name: "foo".to_string(),
            opts: "bar".to_string(),
            comment: "% baz".to_string(),
            is_list_like: true,
        };

        assert_eq!(env_1.latex_begin(), "\\begin{foo}bar % baz");
        assert_eq!(env_1.latex_end(), "\\end{foo}");
        assert_eq!(env_1.is_list_like(), true);
        assert_eq!(env_1.indent_depth(), 0);

        let env_2 = Environment {
            indent_depth: 2,
            name: "abc".to_string(),
            opts: "def".to_string(),
            comment: "".to_string(),
            is_list_like: false,
        };

        assert_eq!(env_2.latex_begin(), "  \\begin{abc}def");
        assert_eq!(env_2.latex_end(), "  \\end{abc}");
        assert_eq!(env_2.is_list_like(), false);
        assert_eq!(env_2.indent_depth(), 2);
    }

    #[test]
    fn list_env_parser() {
        use super::list_env_parser;

        let a = b"itemize";
        let b = b"enumerate*";
        let c = b"    description  *";
        let d = b"item";
        let e = b"foobar";

        assert_eq!(list_env_parser(&a[..]), Done(&b""[..], &a[..]));
        assert_eq!(list_env_parser(&b[..]), Done(&b"*"[..], &b"enumerate"[..]));
        assert_eq!(list_env_parser(&c[..]), Done(&b"*"[..], &b"description"[..]));
        assert_eq!(list_env_parser(&d[..]), Incomplete(Needed::Size(7)));
        assert_eq!(list_env_parser(&e[..]), Error(error_position!(ErrorKind::Alt, &e[..])));
    }

    #[test]
    fn escaped_colon() {
        use super::escaped_colon;

        let a = br"\:";
        let b = b"";
        let c = b"ab";

        assert_eq!(escaped_colon(&a[..]), Done(&b""[..], ':' as u8));
        assert_eq!(escaped_colon(&b[..]), Incomplete(Needed::Size(1)));
        assert_eq!(escaped_colon(&c[..]), Error(error_position!(ErrorKind::Char, &c[..])));
    }

    #[test]
    fn escaped_percent() {
        use super::escaped_percent;

        let a = br"\%";
        let b = b"";
        let c = b"ab";

        assert_eq!(escaped_percent(&a[..]), Done(&b""[..], '%' as u8));
        assert_eq!(escaped_percent(&b[..]), Incomplete(Needed::Size(1)));
        assert_eq!(escaped_percent(&c[..]), Error(error_position!(ErrorKind::Char, &c[..])));
    }

    #[test]
    fn name_parser() {
        use super::name_parser;

        assert_eq!(name_parser(&br"abc"[..]), Done(&b"bc"[..], 'a' as u8));
        assert_eq!(name_parser(&br"\:abc"[..]), Done(&b"abc"[..], ':' as u8));
        assert_eq!(name_parser(&b""[..]), Incomplete(Needed::Size(1)));

        for e in vec![b":E", b"%E", b"(E", b"[E", b"{E", b" E", b"\tE"] {
            assert_eq!(name_parser(&e[..]), Error(error_position!(ErrorKind::Alt, &e[..])));
        }
    }

    #[test]
    fn opts_parser() {
        use super::opts_parser;

        assert_eq!(opts_parser(&br"abc"[..]), Done(&b"bc"[..], 'a' as u8));
        assert_eq!(opts_parser(&br"\:abc"[..]), Done(&b"abc"[..], ':' as u8));
        assert_eq!(opts_parser(&br"\%abc"[..]), Done(&b"abc"[..], '%' as u8));
        assert_eq!(opts_parser(&br"(abc"[..]), Done(&b"abc"[..], '(' as u8));
        assert_eq!(opts_parser(&br"[abc"[..]), Done(&b"abc"[..], '[' as u8));
        assert_eq!(opts_parser(&br" abc"[..]), Done(&b"abc"[..], ' ' as u8));
        assert_eq!(opts_parser(&b""[..]), Incomplete(Needed::Size(1)));

        for e in vec![b":E", b"%E"] {
            assert_eq!(opts_parser(&e[..]), Error(error_position!(ErrorKind::Alt, &e[..])));
        }
    }

    #[test]
    fn args_parser() {
        use super::args_parser;

        assert_eq!(args_parser(&br"abc"[..]), Done(&b"bc"[..], 'a' as u8));
        assert_eq!(args_parser(&br"\:abc"[..]), Done(&b":abc"[..], '\\' as u8));
        assert_eq!(args_parser(&br"\%abc"[..]), Done(&b"abc"[..], '%' as u8));
        assert_eq!(args_parser(&br"(abc"[..]), Done(&b"abc"[..], '(' as u8));
        assert_eq!(args_parser(&br"[abc"[..]), Done(&b"abc"[..], '[' as u8));
        assert_eq!(args_parser(&br" abc"[..]), Done(&b"abc"[..], ' ' as u8));
        assert_eq!(args_parser(&b""[..]), Incomplete(Needed::Size(1)));

        assert_eq!(args_parser(&b"%E"[..]), Error(error_position!(ErrorKind::Alt, &b"%E"[..])));
    }
}
