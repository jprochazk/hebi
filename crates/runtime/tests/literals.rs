#[path = "common/common.rs"]
#[macro_use]
mod common;

check!(none, r#"none"#);
check!(r#true, r#"true"#);
check!(r#false, r#"false"#);
check!(int, r#"1"#);
check!(float, r#"0.1"#);
check!(float_exp, r#"1.5e3"#);
check!(float_pi_n_exp, r#"3.14e-3"#);
check!(string, r#""\tas\\df\u{2800}\x28\n""#);
check!(list, r#"[0, 1, 2]"#);
check!(dict, r#"{a:0, b:1, c:2}"#);
check!(dict_expr_keys, r#"{["a"]:0, ["b"]:1, ["c"]:2}"#);
