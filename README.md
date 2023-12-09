# qrqrpar

![qrqrpar](https://github.com/Nakanishi123/qrqrpar/assets/45790603/35bc5123-1610-48b5-8755-da5a49a4c859)

A QR code generator supporting rMQR

## Example

### Normal Usage

```rust
use qrqrpar::{QrCode, QrStyle};

fn main() {
    // Encode some data into bits.
    let code = QrCode::rmqr("Hello, rmqr!").unwrap();

    // Define style
    let style = QrStyle::default();

    // Render the bits into an image and save it.
    code.save_png("rmqr.png", &style).unwrap();
}
``` 

Generates this image:

![rmqr](https://github.com/Nakanishi123/qrqrpar/assets/45790603/dfd77e9b-de17-4dee-ab02-dce6dcd5d7c8)


### With Options

```rust
use qrqrpar::{EcLevel, QrCode, QrShape, QrStyle, RmqrStrategy};

fn main() {
    // Encode some data into bits with the desired ECC level and strategy.
    let code = QrCode::rmqr_with_options(
        "驫驫驫驫驫驫驫驫驫驫驫驫驫驫",
        EcLevel::H,
        RmqrStrategy::Width,
    )
    .unwrap();

    // Specify the desired output size and style.
    let style = QrStyle::new("#0000cc", "#ffffcc", QrShape::Round, 720, 2.0);

    // Render the bits into an image and save it.
    code.save_svg("rmqr_round.svg", &style).unwrap();
}
``` 
Generates this image:

![rmqr_round](https://github.com/Nakanishi123/qrqrpar/assets/45790603/edd7a2d0-2c34-488c-a7c4-a65b9a930069)

### Normal QR Code


```rust
use qrqrpar::{QrCode, QrStyle};

fn main() {
    let code = QrCode::new("Normal QR code").unwrap();

    // Specify the desired output color.
    let style = QrStyle {
        background_color: String::from("rgba(0,0,0,0)"),
        ..Default::default()
    };

    // Render the bits into an image and save it.
    code.save_png("normal_qr.png", &style).unwrap();
}
```

Generates this image:

[<img src="https://github.com/Nakanishi123/qrqrpar/assets/45790603/6fae782a-27a3-4550-8c39-012216857bc2" width="250"/>](normal_qr.png)

## Derived from

Original library: [qrcode-rust](https://github.com/kennytm/qrcode-rust)

License: Apache-2.0 or MIT 

This library is a derived work based on `qrcode-rust`. It is licensed under Apache-2.0 or MIT License. Please refer to the original library's license for more information.

## License

Licensed under BSD-3-Clause License. See [LICENSE](LICENSE) for more information.