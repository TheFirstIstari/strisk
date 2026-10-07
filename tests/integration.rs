use std::process::Command;

#[test]
fn end_to_end_synthetic_video() {
    let dir = tempfile::tempdir().unwrap();
    let video = dir.path().join("t.mp4");
    let out = dir.path().join("out.png");

    // 2s red then 2s blue at 10fps
    let status = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=red:size=160x120:duration=2:rate=10",
            "-f",
            "lavfi",
            "-i",
            "color=blue:size=160x120:duration=2:rate=10",
            "-filter_complex",
            "[0:v][1:v]concat=n=2:v=1:a=0[v]",
            "-map",
            "[v]",
            "-y",
        ])
        .arg(&video)
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new(env!("CARGO_BIN_EXE_strisk"))
        .arg(&video)
        .args(["-o"])
        .arg(&out)
        .args(["-r", "200"])
        .status()
        .unwrap();
    assert!(status.success());

    let decoder = png::Decoder::new(std::fs::File::open(&out).unwrap());
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!((info.width, info.height), (402, 402));

    let px = |x: usize, y: usize| {
        let o = (y * info.width as usize + x) * 4;
        (buf[o], buf[o + 1], buf[o + 2], buf[o + 3])
    };
    let c = info.width as usize / 2;
    assert_eq!(px(0, 0).3, 0, "corner transparent");
    assert_eq!(px(c, c).3, 0, "center transparent");
    assert_eq!(px(c, 5).3, 0, "notch transparent");
    // 90 deg (right), halfway out: should be reddish (first half of video)
    let right = px(c + 120, c);
    assert!(
        right.0 > 150 && right.2 < 80 && right.3 == 255,
        "got {:?}",
        right
    );
    // 270 deg (left): blueish
    let left = px(c - 120, c);
    assert!(
        left.2 > 150 && left.0 < 80 && left.3 == 255,
        "got {:?}",
        left
    );
}
