//! Verifies that every code example in `README.md` compiles and runs.
#![cfg(all(feature = "canonical", feature = "stream", feature = "indefinite"))]

use const_cbor::canonical::MapBuilder;

#[test]
fn readme_examples() {
    use const_cbor::encode::consts;
    use const_cbor::stream::StreamDecoder;
    use const_cbor::{Decoder, Encoder};

    // Encode
    let mut buf = [0u8; 64];
    let mut enc = Encoder::new(&mut buf);
    enc.array_len(2).unwrap();
    enc.u32(1000).unwrap();
    enc.str("hello").unwrap();
    let n = enc.written();
    assert_eq!(&buf[..n], &[0x82, 0x19, 0x03, 0xe8, 0x65, b'h', b'e', b'l', b'l', b'o']);

    // Decode
    let mut dec = Decoder::new(&buf[..n]);
    assert_eq!(dec.array_len().unwrap(), 2);
    assert_eq!(dec.u32().unwrap(), 1000);
    assert_eq!(dec.str().unwrap(), "hello");
    assert!(dec.is_empty());

    // const encoding
    let neg_ten: ([u8; 2], usize) = {
        let mut b = [0u8; 2];
        let k = consts::put_i64(&mut b, -10);
        (b, k)
    };
    assert_eq!(&neg_ten.0[..neg_ten.1], &[0x29]);

    // indefinite chunks
    #[cfg(feature = "indefinite")]
    {
        let mut buf2 = [0u8; 32];
        let mut e = Encoder::new(&mut buf2);
        e.str_indef(&["strea", "ming"]).unwrap();
        let mut d = Decoder::new(&buf2);
        let mut chunks = d.str_chunks().unwrap();
        assert_eq!(chunks.next(), Some("strea"));
        assert_eq!(chunks.next(), Some("ming"));
        while chunks.next().is_some() {} // pulls the break
        assert!(chunks.finished());
        d.sync_chunks(&chunks).unwrap();

        // canonical
        let mut buf3 = [0u8; 64];
        let mut enc3 = Encoder::new(&mut buf3);
        enc3.map_len(2).unwrap();
        let mut m = MapBuilder::<2>::new(&mut enc3);
        m.add(|k| k.str("bb"), |v| v.u8(1)).unwrap();
        m.add(|k| k.str("aa"), |v| v.u8(2)).unwrap();
        m.finish(&mut [0u8; 32]).unwrap();
        let (out, n3) = enc3.finish();
        let mut dec3 = Decoder::new(&out[..n3]);
        dec3.map_len().unwrap();
        assert_eq!(dec3.str().unwrap(), "aa"); // canonical order
        assert_eq!(dec3.u8().unwrap(), 2);

        // stream
        let mut scratch = [0u8; 256];
        let mut stream = StreamDecoder::new(&mut scratch);
        stream.feed(&[0x01, 0x02]);
        if let Ok(Some(mut item)) = stream.take_item() {
            assert_eq!(item.u8().unwrap(), 1);
        }
    }
}
