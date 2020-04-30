// This file was generated by gir (https://github.com/gtk-rs/gir @ d1e88f9)
// from gir-files (https://github.com/gtk-rs/gir-files @ 65dbff8)
// DO NOT EDIT

extern crate gstreamer_rtsp_sys;
extern crate shell_words;
extern crate tempfile;
use gstreamer_rtsp_sys::*;
use std::env;
use std::error::Error;
use std::mem::{align_of, size_of};
use std::path::Path;
use std::process::Command;
use std::str;
use tempfile::Builder;

static PACKAGES: &[&str] = &["gstreamer-rtsp-1.0"];

#[derive(Clone, Debug)]
struct Compiler {
    pub args: Vec<String>,
}

impl Compiler {
    pub fn new() -> Result<Compiler, Box<dyn Error>> {
        let mut args = get_var("CC", "cc")?;
        args.push("-Wno-deprecated-declarations".to_owned());
        // For %z support in printf when using MinGW.
        args.push("-D__USE_MINGW_ANSI_STDIO".to_owned());
        args.extend(get_var("CFLAGS", "")?);
        args.extend(get_var("CPPFLAGS", "")?);
        args.extend(pkg_config_cflags(PACKAGES)?);
        Ok(Compiler { args })
    }

    pub fn define<'a, V: Into<Option<&'a str>>>(&mut self, var: &str, val: V) {
        let arg = match val.into() {
            None => format!("-D{}", var),
            Some(val) => format!("-D{}={}", var, val),
        };
        self.args.push(arg);
    }

    pub fn compile(&self, src: &Path, out: &Path) -> Result<(), Box<dyn Error>> {
        let mut cmd = self.to_command();
        cmd.arg(src);
        cmd.arg("-o");
        cmd.arg(out);
        let status = cmd.spawn()?.wait()?;
        if !status.success() {
            return Err(format!("compilation command {:?} failed, {}", &cmd, status).into());
        }
        Ok(())
    }

    fn to_command(&self) -> Command {
        let mut cmd = Command::new(&self.args[0]);
        cmd.args(&self.args[1..]);
        cmd
    }
}

fn get_var(name: &str, default: &str) -> Result<Vec<String>, Box<dyn Error>> {
    match env::var(name) {
        Ok(value) => Ok(shell_words::split(&value)?),
        Err(env::VarError::NotPresent) => Ok(shell_words::split(default)?),
        Err(err) => Err(format!("{} {}", name, err).into()),
    }
}

fn pkg_config_cflags(packages: &[&str]) -> Result<Vec<String>, Box<dyn Error>> {
    if packages.is_empty() {
        return Ok(Vec::new());
    }
    let mut cmd = Command::new("pkg-config");
    cmd.arg("--cflags");
    cmd.args(packages);
    let out = cmd.output()?;
    if !out.status.success() {
        return Err(format!("command {:?} returned {}", &cmd, out.status).into());
    }
    let stdout = str::from_utf8(&out.stdout)?;
    Ok(shell_words::split(stdout.trim())?)
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct Layout {
    size: usize,
    alignment: usize,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
struct Results {
    /// Number of successfully completed tests.
    passed: usize,
    /// Total number of failed tests (including those that failed to compile).
    failed: usize,
    /// Number of tests that failed to compile.
    failed_to_compile: usize,
}

impl Results {
    fn record_passed(&mut self) {
        self.passed += 1;
    }
    fn record_failed(&mut self) {
        self.failed += 1;
    }
    fn record_failed_to_compile(&mut self) {
        self.failed += 1;
        self.failed_to_compile += 1;
    }
    fn summary(&self) -> String {
        format!(
            "{} passed; {} failed (compilation errors: {})",
            self.passed, self.failed, self.failed_to_compile
        )
    }
    fn expect_total_success(&self) {
        if self.failed == 0 {
            println!("OK: {}", self.summary());
        } else {
            panic!("FAILED: {}", self.summary());
        };
    }
}

#[test]
fn cross_validate_constants_with_c() {
    let tmpdir = Builder::new()
        .prefix("abi")
        .tempdir()
        .expect("temporary directory");
    let cc = Compiler::new().expect("configured compiler");

    assert_eq!(
        "1",
        get_c_value(tmpdir.path(), &cc, "1").expect("C constant"),
        "failed to obtain correct constant value for 1"
    );

    let mut results: Results = Default::default();
    for (i, &(name, rust_value)) in RUST_CONSTANTS.iter().enumerate() {
        match get_c_value(tmpdir.path(), &cc, name) {
            Err(e) => {
                results.record_failed_to_compile();
                eprintln!("{}", e);
            }
            Ok(ref c_value) => {
                if rust_value == c_value {
                    results.record_passed();
                } else {
                    results.record_failed();
                    eprintln!(
                        "Constant value mismatch for {}\nRust: {:?}\nC:    {:?}",
                        name, rust_value, c_value
                    );
                }
            }
        };
        if (i + 1) % 25 == 0 {
            println!("constants ... {}", results.summary());
        }
    }
    results.expect_total_success();
}

#[test]
fn cross_validate_layout_with_c() {
    let tmpdir = Builder::new()
        .prefix("abi")
        .tempdir()
        .expect("temporary directory");
    let cc = Compiler::new().expect("configured compiler");

    assert_eq!(
        Layout {
            size: 1,
            alignment: 1
        },
        get_c_layout(tmpdir.path(), &cc, "char").expect("C layout"),
        "failed to obtain correct layout for char type"
    );

    let mut results: Results = Default::default();
    for (i, &(name, rust_layout)) in RUST_LAYOUTS.iter().enumerate() {
        match get_c_layout(tmpdir.path(), &cc, name) {
            Err(e) => {
                results.record_failed_to_compile();
                eprintln!("{}", e);
            }
            Ok(c_layout) => {
                if rust_layout == c_layout {
                    results.record_passed();
                } else {
                    results.record_failed();
                    eprintln!(
                        "Layout mismatch for {}\nRust: {:?}\nC:    {:?}",
                        name, rust_layout, &c_layout
                    );
                }
            }
        };
        if (i + 1) % 25 == 0 {
            println!("layout    ... {}", results.summary());
        }
    }
    results.expect_total_success();
}

fn get_c_layout(dir: &Path, cc: &Compiler, name: &str) -> Result<Layout, Box<dyn Error>> {
    let exe = dir.join("layout");
    let mut cc = cc.clone();
    cc.define("ABI_TYPE_NAME", name);
    cc.compile(Path::new("tests/layout.c"), &exe)?;

    let mut abi_cmd = Command::new(exe);
    let output = abi_cmd.output()?;
    if !output.status.success() {
        return Err(format!("command {:?} failed, {:?}", &abi_cmd, &output).into());
    }

    let stdout = str::from_utf8(&output.stdout)?;
    let mut words = stdout.trim().split_whitespace();
    let size = words.next().unwrap().parse().unwrap();
    let alignment = words.next().unwrap().parse().unwrap();
    Ok(Layout { size, alignment })
}

fn get_c_value(dir: &Path, cc: &Compiler, name: &str) -> Result<String, Box<dyn Error>> {
    let exe = dir.join("constant");
    let mut cc = cc.clone();
    cc.define("ABI_CONSTANT_NAME", name);
    cc.compile(Path::new("tests/constant.c"), &exe)?;

    let mut abi_cmd = Command::new(exe);
    let output = abi_cmd.output()?;
    if !output.status.success() {
        return Err(format!("command {:?} failed, {:?}", &abi_cmd, &output).into());
    }

    let output = str::from_utf8(&output.stdout)?.trim();
    if !output.starts_with("###gir test###") || !output.ends_with("###gir test###") {
        return Err(format!(
            "command {:?} return invalid output, {:?}",
            &abi_cmd, &output
        )
        .into());
    }

    Ok(String::from(&output[14..(output.len() - 14)]))
}

const RUST_LAYOUTS: &[(&str, Layout)] = &[
    (
        "GstRTSPAuthCredential",
        Layout {
            size: size_of::<GstRTSPAuthCredential>(),
            alignment: align_of::<GstRTSPAuthCredential>(),
        },
    ),
    (
        "GstRTSPAuthMethod",
        Layout {
            size: size_of::<GstRTSPAuthMethod>(),
            alignment: align_of::<GstRTSPAuthMethod>(),
        },
    ),
    (
        "GstRTSPAuthParam",
        Layout {
            size: size_of::<GstRTSPAuthParam>(),
            alignment: align_of::<GstRTSPAuthParam>(),
        },
    ),
    (
        "GstRTSPEvent",
        Layout {
            size: size_of::<GstRTSPEvent>(),
            alignment: align_of::<GstRTSPEvent>(),
        },
    ),
    (
        "GstRTSPExtensionInterface",
        Layout {
            size: size_of::<GstRTSPExtensionInterface>(),
            alignment: align_of::<GstRTSPExtensionInterface>(),
        },
    ),
    (
        "GstRTSPFamily",
        Layout {
            size: size_of::<GstRTSPFamily>(),
            alignment: align_of::<GstRTSPFamily>(),
        },
    ),
    (
        "GstRTSPHeaderField",
        Layout {
            size: size_of::<GstRTSPHeaderField>(),
            alignment: align_of::<GstRTSPHeaderField>(),
        },
    ),
    (
        "GstRTSPLowerTrans",
        Layout {
            size: size_of::<GstRTSPLowerTrans>(),
            alignment: align_of::<GstRTSPLowerTrans>(),
        },
    ),
    (
        "GstRTSPMessage",
        Layout {
            size: size_of::<GstRTSPMessage>(),
            alignment: align_of::<GstRTSPMessage>(),
        },
    ),
    (
        "GstRTSPMethod",
        Layout {
            size: size_of::<GstRTSPMethod>(),
            alignment: align_of::<GstRTSPMethod>(),
        },
    ),
    (
        "GstRTSPMsgType",
        Layout {
            size: size_of::<GstRTSPMsgType>(),
            alignment: align_of::<GstRTSPMsgType>(),
        },
    ),
    (
        "GstRTSPProfile",
        Layout {
            size: size_of::<GstRTSPProfile>(),
            alignment: align_of::<GstRTSPProfile>(),
        },
    ),
    (
        "GstRTSPRange",
        Layout {
            size: size_of::<GstRTSPRange>(),
            alignment: align_of::<GstRTSPRange>(),
        },
    ),
    (
        "GstRTSPRangeUnit",
        Layout {
            size: size_of::<GstRTSPRangeUnit>(),
            alignment: align_of::<GstRTSPRangeUnit>(),
        },
    ),
    (
        "GstRTSPResult",
        Layout {
            size: size_of::<GstRTSPResult>(),
            alignment: align_of::<GstRTSPResult>(),
        },
    ),
    (
        "GstRTSPState",
        Layout {
            size: size_of::<GstRTSPState>(),
            alignment: align_of::<GstRTSPState>(),
        },
    ),
    (
        "GstRTSPStatusCode",
        Layout {
            size: size_of::<GstRTSPStatusCode>(),
            alignment: align_of::<GstRTSPStatusCode>(),
        },
    ),
    (
        "GstRTSPTime",
        Layout {
            size: size_of::<GstRTSPTime>(),
            alignment: align_of::<GstRTSPTime>(),
        },
    ),
    (
        "GstRTSPTime2",
        Layout {
            size: size_of::<GstRTSPTime2>(),
            alignment: align_of::<GstRTSPTime2>(),
        },
    ),
    (
        "GstRTSPTimeRange",
        Layout {
            size: size_of::<GstRTSPTimeRange>(),
            alignment: align_of::<GstRTSPTimeRange>(),
        },
    ),
    (
        "GstRTSPTimeType",
        Layout {
            size: size_of::<GstRTSPTimeType>(),
            alignment: align_of::<GstRTSPTimeType>(),
        },
    ),
    (
        "GstRTSPTransMode",
        Layout {
            size: size_of::<GstRTSPTransMode>(),
            alignment: align_of::<GstRTSPTransMode>(),
        },
    ),
    (
        "GstRTSPTransport",
        Layout {
            size: size_of::<GstRTSPTransport>(),
            alignment: align_of::<GstRTSPTransport>(),
        },
    ),
    (
        "GstRTSPUrl",
        Layout {
            size: size_of::<GstRTSPUrl>(),
            alignment: align_of::<GstRTSPUrl>(),
        },
    ),
    (
        "GstRTSPVersion",
        Layout {
            size: size_of::<GstRTSPVersion>(),
            alignment: align_of::<GstRTSPVersion>(),
        },
    ),
    (
        "GstRTSPWatchFuncs",
        Layout {
            size: size_of::<GstRTSPWatchFuncs>(),
            alignment: align_of::<GstRTSPWatchFuncs>(),
        },
    ),
];

const RUST_CONSTANTS: &[(&str, &str)] = &[
    ("(guint) GST_RTSP_ANNOUNCE", "2"),
    ("(gint) GST_RTSP_AUTH_BASIC", "1"),
    ("(gint) GST_RTSP_AUTH_DIGEST", "2"),
    ("(gint) GST_RTSP_AUTH_NONE", "0"),
    ("GST_RTSP_DEFAULT_PORT", "554"),
    ("(guint) GST_RTSP_DESCRIBE", "1"),
    ("(gint) GST_RTSP_EEOF", "-11"),
    ("(gint) GST_RTSP_EINTR", "-3"),
    ("(gint) GST_RTSP_EINVAL", "-2"),
    ("(gint) GST_RTSP_ELAST", "-17"),
    ("(gint) GST_RTSP_ENET", "-12"),
    ("(gint) GST_RTSP_ENOMEM", "-4"),
    ("(gint) GST_RTSP_ENOTIMPL", "-6"),
    ("(gint) GST_RTSP_ENOTIP", "-13"),
    ("(gint) GST_RTSP_EPARSE", "-8"),
    ("(gint) GST_RTSP_ERESOLV", "-5"),
    ("(gint) GST_RTSP_ERROR", "-1"),
    ("(gint) GST_RTSP_ESYS", "-7"),
    ("(gint) GST_RTSP_ETGET", "-15"),
    ("(gint) GST_RTSP_ETIMEOUT", "-14"),
    ("(gint) GST_RTSP_ETPOST", "-16"),
    ("(guint) GST_RTSP_EV_READ", "1"),
    ("(guint) GST_RTSP_EV_WRITE", "2"),
    ("(gint) GST_RTSP_EWSASTART", "-9"),
    ("(gint) GST_RTSP_EWSAVERSION", "-10"),
    ("(gint) GST_RTSP_FAM_INET", "1"),
    ("(gint) GST_RTSP_FAM_INET6", "2"),
    ("(gint) GST_RTSP_FAM_NONE", "0"),
    ("(guint) GST_RTSP_GET", "2048"),
    ("(guint) GST_RTSP_GET_PARAMETER", "4"),
    ("(gint) GST_RTSP_HDR_ACCEPT", "1"),
    ("(gint) GST_RTSP_HDR_ACCEPT_CHARSET", "56"),
    ("(gint) GST_RTSP_HDR_ACCEPT_ENCODING", "2"),
    ("(gint) GST_RTSP_HDR_ACCEPT_LANGUAGE", "3"),
    ("(gint) GST_RTSP_HDR_ACCEPT_RANGES", "86"),
    ("(gint) GST_RTSP_HDR_ALERT", "45"),
    ("(gint) GST_RTSP_HDR_ALLOW", "4"),
    ("(gint) GST_RTSP_HDR_AUTHENTICATION_INFO", "76"),
    ("(gint) GST_RTSP_HDR_AUTHORIZATION", "5"),
    ("(gint) GST_RTSP_HDR_BANDWIDTH", "6"),
    ("(gint) GST_RTSP_HDR_BLOCKSIZE", "7"),
    ("(gint) GST_RTSP_HDR_CACHE_CONTROL", "8"),
    ("(gint) GST_RTSP_HDR_CLIENT_CHALLENGE", "40"),
    ("(gint) GST_RTSP_HDR_CLIENT_ID", "46"),
    ("(gint) GST_RTSP_HDR_COMPANY_ID", "47"),
    ("(gint) GST_RTSP_HDR_CONFERENCE", "9"),
    ("(gint) GST_RTSP_HDR_CONNECTION", "10"),
    ("(gint) GST_RTSP_HDR_CONTENT_BASE", "11"),
    ("(gint) GST_RTSP_HDR_CONTENT_ENCODING", "12"),
    ("(gint) GST_RTSP_HDR_CONTENT_LANGUAGE", "13"),
    ("(gint) GST_RTSP_HDR_CONTENT_LENGTH", "14"),
    ("(gint) GST_RTSP_HDR_CONTENT_LOCATION", "15"),
    ("(gint) GST_RTSP_HDR_CONTENT_TYPE", "16"),
    ("(gint) GST_RTSP_HDR_CSEQ", "17"),
    ("(gint) GST_RTSP_HDR_DATE", "18"),
    ("(gint) GST_RTSP_HDR_ETAG", "54"),
    ("(gint) GST_RTSP_HDR_EXPIRES", "19"),
    ("(gint) GST_RTSP_HDR_FRAMES", "87"),
    ("(gint) GST_RTSP_HDR_FROM", "20"),
    ("(gint) GST_RTSP_HDR_GUID", "48"),
    ("(gint) GST_RTSP_HDR_HOST", "77"),
    ("(gint) GST_RTSP_HDR_IF_MATCH", "55"),
    ("(gint) GST_RTSP_HDR_IF_MODIFIED_SINCE", "21"),
    ("(gint) GST_RTSP_HDR_INVALID", "0"),
    ("(gint) GST_RTSP_HDR_KEYMGMT", "82"),
    ("(gint) GST_RTSP_HDR_LANGUAGE", "51"),
    ("(gint) GST_RTSP_HDR_LAST", "89"),
    ("(gint) GST_RTSP_HDR_LAST_MODIFIED", "22"),
    ("(gint) GST_RTSP_HDR_LOCATION", "53"),
    ("(gint) GST_RTSP_HDR_MAX_ASM_WIDTH", "50"),
    ("(gint) GST_RTSP_HDR_MEDIA_PROPERTIES", "84"),
    ("(gint) GST_RTSP_HDR_PIPELINED_REQUESTS", "83"),
    ("(gint) GST_RTSP_HDR_PLAYER_START_TIME", "52"),
    ("(gint) GST_RTSP_HDR_PRAGMA", "78"),
    ("(gint) GST_RTSP_HDR_PROXY_AUTHENTICATE", "23"),
    ("(gint) GST_RTSP_HDR_PROXY_REQUIRE", "24"),
    ("(gint) GST_RTSP_HDR_PUBLIC", "25"),
    ("(gint) GST_RTSP_HDR_RANGE", "26"),
    ("(gint) GST_RTSP_HDR_RATE_CONTROL", "88"),
    ("(gint) GST_RTSP_HDR_REAL_CHALLENGE1", "41"),
    ("(gint) GST_RTSP_HDR_REAL_CHALLENGE2", "42"),
    ("(gint) GST_RTSP_HDR_REAL_CHALLENGE3", "43"),
    ("(gint) GST_RTSP_HDR_REFERER", "27"),
    ("(gint) GST_RTSP_HDR_REGION_DATA", "49"),
    ("(gint) GST_RTSP_HDR_REQUIRE", "28"),
    ("(gint) GST_RTSP_HDR_RETRY_AFTER", "29"),
    ("(gint) GST_RTSP_HDR_RTCP_INTERVAL", "81"),
    ("(gint) GST_RTSP_HDR_RTP_INFO", "30"),
    ("(gint) GST_RTSP_HDR_SCALE", "31"),
    ("(gint) GST_RTSP_HDR_SEEK_STYLE", "85"),
    ("(gint) GST_RTSP_HDR_SERVER", "33"),
    ("(gint) GST_RTSP_HDR_SESSION", "32"),
    ("(gint) GST_RTSP_HDR_SPEED", "34"),
    ("(gint) GST_RTSP_HDR_SUBSCRIBE", "44"),
    ("(gint) GST_RTSP_HDR_SUPPORTED", "57"),
    ("(gint) GST_RTSP_HDR_TIMESTAMP", "75"),
    ("(gint) GST_RTSP_HDR_TRANSPORT", "35"),
    ("(gint) GST_RTSP_HDR_UNSUPPORTED", "36"),
    ("(gint) GST_RTSP_HDR_USER_AGENT", "37"),
    ("(gint) GST_RTSP_HDR_VARY", "58"),
    ("(gint) GST_RTSP_HDR_VIA", "38"),
    ("(gint) GST_RTSP_HDR_WWW_AUTHENTICATE", "39"),
    ("(gint) GST_RTSP_HDR_X_ACCELERATE_STREAMING", "59"),
    ("(gint) GST_RTSP_HDR_X_ACCEPT_AUTHENT", "60"),
    ("(gint) GST_RTSP_HDR_X_ACCEPT_PROXY_AUTHENT", "61"),
    ("(gint) GST_RTSP_HDR_X_BROADCAST_ID", "62"),
    ("(gint) GST_RTSP_HDR_X_BURST_STREAMING", "63"),
    ("(gint) GST_RTSP_HDR_X_NOTICE", "64"),
    ("(gint) GST_RTSP_HDR_X_PLAYER_LAG_TIME", "65"),
    ("(gint) GST_RTSP_HDR_X_PLAYLIST", "66"),
    ("(gint) GST_RTSP_HDR_X_PLAYLIST_CHANGE_NOTICE", "67"),
    ("(gint) GST_RTSP_HDR_X_PLAYLIST_GEN_ID", "68"),
    ("(gint) GST_RTSP_HDR_X_PLAYLIST_SEEK_ID", "69"),
    ("(gint) GST_RTSP_HDR_X_PROXY_CLIENT_AGENT", "70"),
    ("(gint) GST_RTSP_HDR_X_PROXY_CLIENT_VERB", "71"),
    ("(gint) GST_RTSP_HDR_X_RECEDING_PLAYLISTCHANGE", "72"),
    ("(gint) GST_RTSP_HDR_X_RTP_INFO", "73"),
    ("(gint) GST_RTSP_HDR_X_SERVER_IP_ADDRESS", "79"),
    ("(gint) GST_RTSP_HDR_X_SESSIONCOOKIE", "80"),
    ("(gint) GST_RTSP_HDR_X_STARTUPPROFILE", "74"),
    ("(guint) GST_RTSP_INVALID", "0"),
    ("(guint) GST_RTSP_LOWER_TRANS_HTTP", "16"),
    ("(guint) GST_RTSP_LOWER_TRANS_TCP", "4"),
    ("(guint) GST_RTSP_LOWER_TRANS_TLS", "32"),
    ("(guint) GST_RTSP_LOWER_TRANS_UDP", "1"),
    ("(guint) GST_RTSP_LOWER_TRANS_UDP_MCAST", "2"),
    ("(guint) GST_RTSP_LOWER_TRANS_UNKNOWN", "0"),
    ("(gint) GST_RTSP_MESSAGE_DATA", "5"),
    ("(gint) GST_RTSP_MESSAGE_HTTP_REQUEST", "3"),
    ("(gint) GST_RTSP_MESSAGE_HTTP_RESPONSE", "4"),
    ("(gint) GST_RTSP_MESSAGE_INVALID", "0"),
    ("(gint) GST_RTSP_MESSAGE_REQUEST", "1"),
    ("(gint) GST_RTSP_MESSAGE_RESPONSE", "2"),
    ("(gint) GST_RTSP_OK", "0"),
    ("(guint) GST_RTSP_OPTIONS", "8"),
    ("(guint) GST_RTSP_PAUSE", "16"),
    ("(guint) GST_RTSP_PLAY", "32"),
    ("(guint) GST_RTSP_POST", "4096"),
    ("(guint) GST_RTSP_PROFILE_AVP", "1"),
    ("(guint) GST_RTSP_PROFILE_AVPF", "4"),
    ("(guint) GST_RTSP_PROFILE_SAVP", "2"),
    ("(guint) GST_RTSP_PROFILE_SAVPF", "8"),
    ("(guint) GST_RTSP_PROFILE_UNKNOWN", "0"),
    ("(gint) GST_RTSP_RANGE_CLOCK", "4"),
    ("(gint) GST_RTSP_RANGE_NPT", "3"),
    ("(gint) GST_RTSP_RANGE_SMPTE", "0"),
    ("(gint) GST_RTSP_RANGE_SMPTE_25", "2"),
    ("(gint) GST_RTSP_RANGE_SMPTE_30_DROP", "1"),
    ("(guint) GST_RTSP_RECORD", "64"),
    ("(guint) GST_RTSP_REDIRECT", "128"),
    ("(guint) GST_RTSP_SETUP", "256"),
    ("(guint) GST_RTSP_SET_PARAMETER", "512"),
    ("(gint) GST_RTSP_STATE_INIT", "1"),
    ("(gint) GST_RTSP_STATE_INVALID", "0"),
    ("(gint) GST_RTSP_STATE_PLAYING", "4"),
    ("(gint) GST_RTSP_STATE_READY", "2"),
    ("(gint) GST_RTSP_STATE_RECORDING", "5"),
    ("(gint) GST_RTSP_STATE_SEEKING", "3"),
    ("(gint) GST_RTSP_STS_AGGREGATE_OPERATION_NOT_ALLOWED", "459"),
    ("(gint) GST_RTSP_STS_BAD_GATEWAY", "502"),
    ("(gint) GST_RTSP_STS_BAD_REQUEST", "400"),
    ("(gint) GST_RTSP_STS_CONFERENCE_NOT_FOUND", "452"),
    ("(gint) GST_RTSP_STS_CONTINUE", "100"),
    ("(gint) GST_RTSP_STS_CREATED", "201"),
    ("(gint) GST_RTSP_STS_DESTINATION_UNREACHABLE", "462"),
    ("(gint) GST_RTSP_STS_FORBIDDEN", "403"),
    ("(gint) GST_RTSP_STS_GATEWAY_TIMEOUT", "504"),
    ("(gint) GST_RTSP_STS_GONE", "410"),
    (
        "(gint) GST_RTSP_STS_HEADER_FIELD_NOT_VALID_FOR_RESOURCE",
        "456",
    ),
    ("(gint) GST_RTSP_STS_INTERNAL_SERVER_ERROR", "500"),
    ("(gint) GST_RTSP_STS_INVALID", "0"),
    ("(gint) GST_RTSP_STS_INVALID_RANGE", "457"),
    ("(gint) GST_RTSP_STS_KEY_MANAGEMENT_FAILURE", "463"),
    ("(gint) GST_RTSP_STS_LENGTH_REQUIRED", "411"),
    ("(gint) GST_RTSP_STS_LOW_ON_STORAGE", "250"),
    ("(gint) GST_RTSP_STS_METHOD_NOT_ALLOWED", "405"),
    ("(gint) GST_RTSP_STS_METHOD_NOT_VALID_IN_THIS_STATE", "455"),
    ("(gint) GST_RTSP_STS_MOVED_PERMANENTLY", "301"),
    ("(gint) GST_RTSP_STS_MOVE_TEMPORARILY", "302"),
    ("(gint) GST_RTSP_STS_MULTIPLE_CHOICES", "300"),
    ("(gint) GST_RTSP_STS_NOT_ACCEPTABLE", "406"),
    ("(gint) GST_RTSP_STS_NOT_ENOUGH_BANDWIDTH", "453"),
    ("(gint) GST_RTSP_STS_NOT_FOUND", "404"),
    ("(gint) GST_RTSP_STS_NOT_IMPLEMENTED", "501"),
    ("(gint) GST_RTSP_STS_NOT_MODIFIED", "304"),
    ("(gint) GST_RTSP_STS_OK", "200"),
    (
        "(gint) GST_RTSP_STS_ONLY_AGGREGATE_OPERATION_ALLOWED",
        "460",
    ),
    ("(gint) GST_RTSP_STS_OPTION_NOT_SUPPORTED", "551"),
    ("(gint) GST_RTSP_STS_PARAMETER_IS_READONLY", "458"),
    ("(gint) GST_RTSP_STS_PARAMETER_NOT_UNDERSTOOD", "451"),
    ("(gint) GST_RTSP_STS_PAYMENT_REQUIRED", "402"),
    ("(gint) GST_RTSP_STS_PRECONDITION_FAILED", "412"),
    ("(gint) GST_RTSP_STS_PROXY_AUTH_REQUIRED", "407"),
    ("(gint) GST_RTSP_STS_REQUEST_ENTITY_TOO_LARGE", "413"),
    ("(gint) GST_RTSP_STS_REQUEST_TIMEOUT", "408"),
    ("(gint) GST_RTSP_STS_REQUEST_URI_TOO_LARGE", "414"),
    ("(gint) GST_RTSP_STS_RTSP_VERSION_NOT_SUPPORTED", "505"),
    ("(gint) GST_RTSP_STS_SEE_OTHER", "303"),
    ("(gint) GST_RTSP_STS_SERVICE_UNAVAILABLE", "503"),
    ("(gint) GST_RTSP_STS_SESSION_NOT_FOUND", "454"),
    ("(gint) GST_RTSP_STS_UNAUTHORIZED", "401"),
    ("(gint) GST_RTSP_STS_UNSUPPORTED_MEDIA_TYPE", "415"),
    ("(gint) GST_RTSP_STS_UNSUPPORTED_TRANSPORT", "461"),
    ("(gint) GST_RTSP_STS_USE_PROXY", "305"),
    ("(guint) GST_RTSP_TEARDOWN", "1024"),
    ("(gint) GST_RTSP_TIME_END", "2"),
    ("(gint) GST_RTSP_TIME_FRAMES", "3"),
    ("(gint) GST_RTSP_TIME_NOW", "1"),
    ("(gint) GST_RTSP_TIME_SECONDS", "0"),
    ("(gint) GST_RTSP_TIME_UTC", "4"),
    ("(guint) GST_RTSP_TRANS_RDT", "2"),
    ("(guint) GST_RTSP_TRANS_RTP", "1"),
    ("(guint) GST_RTSP_TRANS_UNKNOWN", "0"),
    ("(gint) GST_RTSP_VERSION_1_0", "16"),
    ("(gint) GST_RTSP_VERSION_1_1", "17"),
    ("(gint) GST_RTSP_VERSION_2_0", "32"),
    ("(gint) GST_RTSP_VERSION_INVALID", "0"),
];
