use std::path::Path;
use std::vec::Vec;
use error::IndentexError;


const LINESEP: &'static str = "\n";
const LATEX_TO_INDENTEX_FACTOR: f64 = 1.5;


pub struct TranspileOptions {
    pub flatten_output: bool,
    pub prepend_do_not_edit_notice: bool,
}

// Indentation processing
#[inline]
fn count_left_indent<T: AsRef<str>>(line: T) -> Option<usize> {
    if line.as_ref().is_empty() {
        None
    } else {
        Some(line.as_ref().chars().count() - line.as_ref().trim_left().chars().count())
    }
}

fn scan_indents<T: AsRef<str>>(lines: &[T]) -> Vec<usize> {
    let raw_indents = lines.iter().map(count_left_indent).collect::<Vec<_>>();

    let mut adjusted_indents: Vec<usize> = Vec::with_capacity(raw_indents.len() + 1);
    let mut last_indent: usize = 0;

    for current_indent in raw_indents.iter().rev() {
        adjusted_indents.push(match *current_indent {
            None => last_indent,
            Some(ind) => {
                last_indent = ind;
                ind
            }
        });
    }

    adjusted_indents.reverse();
    adjusted_indents.push(0);

    adjusted_indents
}


// Transpilation
fn transpile<T: AsRef<str>>(lines: &[T], options: &TranspileOptions) -> String {
    use parsers::Environment;
    use parsers::Hashline::{PlainLine, OpenEnv};
    use parsers::process_line;

    // The number of environments is not known beforehand
    let mut env_stack: Vec<Environment> = Vec::new();

    // Input size is the sum of all line lengths plus the number of lines (for lineseps)
    let input_size = lines.iter().fold(0, |sum, l| sum + l.as_ref().len()) + lines.len();
    // We do not know how much larger the transpiled LaTeX file will be, but we can guess...
    let indentex_size = (LATEX_TO_INDENTEX_FACTOR * (input_size as f64)).round() as usize;
    let mut transpiled = String::with_capacity(indentex_size);

    let adjusted_indents = scan_indents(lines);

    if options.prepend_do_not_edit_notice {
        transpiled.push_str("% ============================================================== %\n");
        transpiled.push_str("%                                                                %\n");
        transpiled.push_str("% THIS IS AN AUTOGENERATED FILE. DO NOT EDIT THIS FILE DIRECTLY. %\n");
        transpiled.push_str("%                                                                %\n");
        transpiled.push_str("% ============================================================== %\n");
    }

    for (line_num, line) in lines.iter().enumerate() {
        let list_like_active = match env_stack.last() {
            None => false, // No environment is active at all
            Some(d) => d.is_list_like(),
        };

        let tl = match process_line(line.as_ref(), list_like_active) {
            PlainLine(l) => l,
            OpenEnv(e) => {
                let tag_begin = e.latex_begin();
                env_stack.push(e);
                tag_begin
            }
        };
        if options.flatten_output {
            transpiled.push_str(tl.trim_left());
        } else {
            transpiled.push_str(&tl);
        }
        transpiled.push_str(LINESEP);

        // Check if we are in an environment and close as many as needed
        while match env_stack.last() {
            None => false,
            Some(d) => d.indent_depth() >= adjusted_indents[line_num + 1],
        } {
            // `unwrap()` is safe here since we have already checked if the stack is empty
            let tag_end = env_stack.pop().unwrap().latex_end();
            if options.flatten_output {
                transpiled.push_str(tag_end.trim_left());
            } else {
                transpiled.push_str(&tag_end);
            }
            transpiled.push_str(LINESEP);
        }
    }

    transpiled
}

pub fn transpile_file<T: AsRef<Path>>(path: T, options: &TranspileOptions) -> Result<(), IndentexError> {
    use file_utils::{read_and_trim_lines, rename_indentex_file, write_to_file};

    let lines = read_and_trim_lines(path.as_ref())?;
    let transpiled_text = transpile(&lines, options);
    let path_out = rename_indentex_file(path)?;
    write_to_file(path_out, &transpiled_text)?;

    Ok(())
}


#[cfg(test)]
mod tests {
    #[test]
    fn count_left_indent() {
        use super::count_left_indent;

        assert_eq!(count_left_indent(""), None);
        assert_eq!(count_left_indent("foo"), Some(0));
        assert_eq!(count_left_indent("  bar"), Some(2));
        // We assume that the input has no trailing whitespaces
        // This is not a bug (but not a nice behaviour either)
        assert_eq!(count_left_indent("   "), Some(3));
    }

    #[test]
    fn scan_indents() {
        use super::scan_indents;

        // Always add a zero at the end
        let a = [" a"];
        assert_eq!(scan_indents(&a), [1, 0]);
        assert_eq!(scan_indents(&a).capacity(), 2);
        // Indents are propagated backwards
        let b = ["  b", "b", "", "  b"];
        assert_eq!(scan_indents(&b), [2, 0, 2, 2, 0]);
        assert_eq!(scan_indents(&b).capacity(), 5);
        // We assume that the input has no trailing whitespaces
        // This is not a bug (but not a nice behaviour either)
        let c = ["", "   "];
        assert_eq!(scan_indents(&c), [3, 3, 0]);
        assert_eq!(scan_indents(&c).capacity(), 3);

        let d = ["d", " d", "", " d", "", "   d", "  d", "     d"];
        assert_eq!(scan_indents(&d), [0, 1, 1, 1, 3, 3, 2, 5, 0]);
        assert_eq!(scan_indents(&d).capacity(), 9);
    }

    #[test]
    fn transpile_do_not_edit_notice() {
        use super::{transpile, TranspileOptions};

        let options = TranspileOptions {
            flatten_output: false,
            prepend_do_not_edit_notice: true,
        };

        assert_eq!(transpile(&["Test"], &options),
        "% ============================================================== %\n\
         %                                                                %\n\
         % THIS IS AN AUTOGENERATED FILE. DO NOT EDIT THIS FILE DIRECTLY. %\n\
         %                                                                %\n\
         % ============================================================== %\n\
         Test\n");
    }

    #[test]
    fn transpile_single_line_cmds() {
        use super::{transpile, TranspileOptions};

        let options = TranspileOptions {
            flatten_output: false,
            prepend_do_not_edit_notice: false,
        };

        // Colon
        assert_eq!(transpile(&["# section: Foo bar"], &options),
            "\\section{Foo bar}\n");
        assert_eq!(transpile(&["# section: Foo: bar"], &options),
            "\\section{Foo: bar}\n");
        assert_eq!(transpile(&["# section [Foo bar]: Foo bar"], &options),
            "\\section[Foo bar]{Foo bar}\n");
        assert_eq!(transpile(&["# section [Foo\\: bar]: Foo: bar"], &options),
            "\\section[Foo: bar]{Foo: bar}\n");

        // Asterisk
        assert_eq!(transpile(&["# section* : spam eggs"], &options),
            "\\section*{spam eggs}\n");

        // Percent
        assert_eq!(transpile(&["# section: foo bar % test"], &options),
            "\\section{foo bar} % test\n");
        assert_eq!(transpile(&["# section: \\% baz % test"], &options),
            "\\section{\\% baz} % test\n");
        assert_eq!(transpile(&["# sec%tion: foo bar"], &options),
            "# sec%tion: foo bar\n");
        assert_eq!(transpile(&["# section % baz: foo bar"], &options),
            "# section % baz: foo bar\n");

        // Do not convert
        assert_eq!(transpile(&["# section"], &options),
            "# section\n");
        assert_eq!(transpile(&["#section: Foo"], &options),
            "#section: Foo\n");
        assert_eq!(transpile(&["\\# section"], &options),
            "\\# section\n");
        assert_eq!(transpile(&["%# section"], &options),
            "%# section\n");
    }

    #[test]
    fn transpile_list_like() {
        use super::{transpile, TranspileOptions};

        let options = TranspileOptions {
            flatten_output: false,
            prepend_do_not_edit_notice: false,
        };

        let options_flatten = TranspileOptions {
            flatten_output: true,
            prepend_do_not_edit_notice: false,
        };

        // Do not convert
        assert_eq!(transpile(&["* a"], &options), "* a\n");
        assert_eq!(transpile(&["\\* b"], &options), "\\* b\n");
        assert_eq!(transpile(&["\\\\* c"], &options), "\\\\* c\n");

        // itemize
        assert_eq!(transpile(&[
            "# itemize:",
            "  * яблоки",
            "  * груши",
            "  * абрикосы",
        ], &options),
        "\\begin{itemize}\n  \
        \\item яблоки\n  \
        \\item груши\n  \
        \\item абрикосы\n\
        \\end{itemize}\n");

        assert_eq!(transpile(&[
            "# itemize:",
            "  * яблоки",
            "  * груши",
            "  * абрикосы",
        ], &options_flatten),
        "\\begin{itemize}\n\
        \\item яблоки\n\
        \\item груши\n\
        \\item абрикосы\n\
        \\end{itemize}\n");

        // enumerate
        assert_eq!(transpile(&[
            "# enumerate:",
            "  * Alice",
            "  * Bob",
            "  *",
        ], &options),
        "\\begin{enumerate}\n  \
        \\item Alice\n  \
        \\item Bob\n  \
        \\item\n\
        \\end{enumerate}\n");

        assert_eq!(transpile(&[
            "# enumerate:",
            "  * Alice",
            "  * Bob",
            "  *",
        ], &options_flatten),
        "\\begin{enumerate}\n\
        \\item Alice\n\
        \\item Bob\n\
        \\item\n\
        \\end{enumerate}\n");

         // description
        assert_eq!(transpile(&[
            "# description:",
            "  *[Ä] letter Ä",
            "  *[Ü] letter Ü",
        ], &options),
        "\\begin{description}\n  \
        \\item [Ä] letter Ä\n  \
        \\item [Ü] letter Ü\n\
        \\end{description}\n");

        assert_eq!(transpile(&[
            "# description:",
            "  *[Ä] letter Ä",
            "  *[Ü] letter Ü",
        ], &options_flatten),
        "\\begin{description}\n\
        \\item [Ä] letter Ä\n\
        \\item [Ü] letter Ü\n\
        \\end{description}\n");

        // leveled
        assert_eq!(transpile(&[
            "# itemize:",
            "  * first level, first item",
            "    # itemize:",
            "      * second level, first item",
            "      * second level, second item",
            "  * first level, second item",
            "    # equation:",
            "      * % this should not be converted",
        ], &options),
        "\\begin{itemize}\n  \
        \\item first level, first item\n    \
        \\begin{itemize}\n      \
        \\item second level, first item\n      \
        \\item second level, second item\n    \
        \\end{itemize}\n  \
        \\item first level, second item\n    \
        \\begin{equation}\n      \
        * % this should not be converted\n    \
        \\end{equation}\n\
        \\end{itemize}\n");

        assert_eq!(transpile(&[
            "# itemize:",
            "  * first level, first item",
            "    # itemize:",
            "      * second level, first item",
            "      * second level, second item",
            "  * first level, second item",
            "    # equation:",
            "      * % this should not be converted",
        ], &options_flatten),
        "\\begin{itemize}\n\
        \\item first level, first item\n\
        \\begin{itemize}\n\
        \\item second level, first item\n\
        \\item second level, second item\n\
        \\end{itemize}\n\
        \\item first level, second item\n\
        \\begin{equation}\n\
        * % this should not be converted\n\
        \\end{equation}\n\
        \\end{itemize}\n");

        // escaped asterisk
        assert_eq!(transpile(&[
            "# enumerate:",
            "  * This should be converted to an item",
            "  \\* and this not",
        ], &options),
        "\\begin{enumerate}\n  \
        \\item This should be converted to an item\n  \
        \\* and this not\n\
        \\end{enumerate}\n");

        assert_eq!(transpile(&[
            "# enumerate:",
            "  * This should be converted to an item",
            "  \\* and this not",
        ], &options_flatten),
        "\\begin{enumerate}\n\
        \\item This should be converted to an item\n\
        \\* and this not\n\
        \\end{enumerate}\n");
    }

    #[test]
    fn transpile_envs() {
        use super::{transpile, TranspileOptions};

        let options = TranspileOptions {
            flatten_output: false,
            prepend_do_not_edit_notice: false,
        };

        let options_flatten = TranspileOptions {
            flatten_output: true,
            prepend_do_not_edit_notice: false,
        };

        // equation environment
        assert_eq!(transpile(&[
            "# equation*:",
            "  # label: eq:test",
            "  a + b",
        ], &options),
        "\\begin{equation*}\n  \
        \\label{eq:test}\n  \
        a + b\n\
        \\end{equation*}\n");

        assert_eq!(transpile(&[
            "# equation*:",
            "  # label: eq:test",
            "  a + b",
        ], &options_flatten),
        "\\begin{equation*}\n\
        \\label{eq:test}\n\
        a + b\n\
        \\end{equation*}\n");

        // tikz environment
        assert_eq!(transpile(&[
            "# tikzpicture [x = 2 cm]:",
            "  \\draw (0, 0) -- (1, 1);",
        ], &options),
        "\\begin{tikzpicture}[x = 2 cm]\n  \
        \\draw (0, 0) -- (1, 1);\n\
        \\end{tikzpicture}\n");

        assert_eq!(transpile(&[
            "# tikzpicture [x = 2 cm]:",
            "  \\draw (0, 0) -- (1, 1);",
        ], &options_flatten),
        "\\begin{tikzpicture}[x = 2 cm]\n\
        \\draw (0, 0) -- (1, 1);\n\
        \\end{tikzpicture}\n");

        // comments
        assert_eq!(transpile(&[
            "# equation: % test",
            "  a + b",
            "# remark [test percent escaping \\%]: % baz",
            "  foo bar",
        ], &options),
        "\\begin{equation} % test\n  \
        a + b\n\
        \\end{equation}\n\
        \\begin{remark}[test percent escaping \\%] % baz\n  \
        foo bar\n\
        \\end{remark}\n");

        assert_eq!(transpile(&[
            "# equation: % test",
            "  a + b",
            "# remark [test percent escaping \\%]: % baz",
            "  foo bar",
        ], &options_flatten),
        "\\begin{equation} % test\n\
        a + b\n\
        \\end{equation}\n\
        \\begin{remark}[test percent escaping \\%] % baz\n\
        foo bar\n\
        \\end{remark}\n");

        // deeply nested environments
        assert_eq!(transpile(&[
            "# a:",
            "  # b:",
            "    # c:",
            "      # d:",
            "        # e:",
            "          # f:",
            "            # g:",
            "              # h:",
            "                # i:",
            "                  foobar",
        ], &options),
        "\\begin{a}\n  \
        \\begin{b}\n    \
        \\begin{c}\n      \
        \\begin{d}\n        \
        \\begin{e}\n          \
        \\begin{f}\n            \
        \\begin{g}\n              \
        \\begin{h}\n                \
        \\begin{i}\n                  \
        foobar\n                \
        \\end{i}\n              \
        \\end{h}\n            \
        \\end{g}\n          \
        \\end{f}\n        \
        \\end{e}\n      \
        \\end{d}\n    \
        \\end{c}\n  \
        \\end{b}\n\
        \\end{a}\n");

        assert_eq!(transpile(&[
            "# a:",
            "  # b:",
            "    # c:",
            "      # d:",
            "        # e:",
            "          # f:",
            "            # g:",
            "              # h:",
            "                # i:",
            "                  foobar",
        ], &options_flatten),
        "\\begin{a}\n\
        \\begin{b}\n\
        \\begin{c}\n\
        \\begin{d}\n\
        \\begin{e}\n\
        \\begin{f}\n\
        \\begin{g}\n\
        \\begin{h}\n\
        \\begin{i}\n\
        foobar\n\
        \\end{i}\n\
        \\end{h}\n\
        \\end{g}\n\
        \\end{f}\n\
        \\end{e}\n\
        \\end{d}\n\
        \\end{c}\n\
        \\end{b}\n\
        \\end{a}\n");
    }

    #[test]
    fn transpile_different_indents() {
        use super::{transpile, TranspileOptions};

        let options = TranspileOptions {
            flatten_output: false,
            prepend_do_not_edit_notice: false,
        };

        // 2 spaces
        assert_eq!(transpile(&[
            "# a:",
            "  # b:",
            "    test_b",
            "  # c:",
            "    test_c"
        ], &options),
        "\\begin{a}\n  \
        \\begin{b}\n    \
        test_b\n  \
        \\end{b}\n  \
        \\begin{c}\n    \
        test_c\n  \
        \\end{c}\n\
        \\end{a}\n");

        // 4 spaces
        assert_eq!(transpile(&[
            "# a:",
            "    # b:",
            "        test_b",
            "    # c:",
            "        test_c"
        ], &options),
        "\\begin{a}\n    \
        \\begin{b}\n        \
        test_b\n    \
        \\end{b}\n    \
        \\begin{c}\n        \
        test_c\n    \
        \\end{c}\n\
        \\end{a}\n");
    }
}
