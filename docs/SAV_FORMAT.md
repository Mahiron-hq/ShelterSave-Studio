# Fallout Shelter .sav: decryption and encryption spec

## 1. Format summary

A `.sav` file is a text file. Its content is a single Base64 string.
Decoded from Base64, it is AES-256-CBC ciphertext with PKCS7 padding.
Decrypted, it is UTF-8 JSON text (the vault save data).

    .sav (text) --Base64 decode--> ciphertext --AES-256-CBC decrypt--> UTF-8 JSON

The key and IV are fixed and the same for every save. There is no signature,
checksum or header. The game accepts any file encrypted with this key and IV
that contains valid JSON in the expected structure.

## 2. Parameters

| Parameter | Value |
|-----------|-------|
| Cipher    | AES-256 |
| Mode      | CBC |
| Padding   | PKCS7 (PKCS#5) |
| Key size  | 32 bytes |
| IV size   | 16 bytes |
| Encoding  | Base64, standard alphabet (`A-Za-z0-9+/`), `=` padding |
| Text      | UTF-8 |

### Key (32 bytes, hex)

    a7ca9f3366d892c2f0bef417341ca971b69ae9f7bacccffcf43c62d1d7d021f9

Origin: eight unsigned 32-bit integers, each written as 4 bytes big-endian,
in this order:

    2815074099, 1725469378, 4039046167, 874293617,
    3063605751, 3133984764, 4097598161, 3620741625

### IV (16 bytes, hex)

    7475383967656a693334307438397532

This is the ASCII string `tu89geji340t89u2`.

## 3. Decryption algorithm

1. Read the file as text. Strip leading and trailing whitespace (spaces, CR, LF).
2. Base64-decode it into bytes. Fail if the string is not valid Base64.
3. Check that the byte length is a non-zero multiple of 16.
4. Decrypt with AES-256-CBC using the key and IV above.
5. Remove PKCS7 padding. Fail if the padding is invalid (wrong key or damaged file).
6. Decode the bytes as UTF-8. The result is the JSON text.
7. Parse the JSON. Fail if it is not valid JSON.

## 4. Encryption algorithm

1. Serialize the save object to JSON text. Compact form (no extra spaces or
   line breaks) is what the original web editor writes.
2. Encode the text as UTF-8 bytes.
3. Add PKCS7 padding up to a multiple of 16 bytes.
4. Encrypt with AES-256-CBC using the key and IV above.
5. Base64-encode the ciphertext (standard alphabet, with `=` padding).
6. Write the Base64 string to the `.sav` file as plain text, with no line breaks.

## 5. Test vector

Plaintext (UTF-8, 7 bytes): `{"a":1}`

Base64 .sav content: `u9bcSUF3rVpSfbPAFIiDjw==`

Decrypting that string must give exactly `{"a":1}`, and encrypting
`{"a":1}` must give exactly that string. The vector was produced and checked
with an independent AES implementation (Node.js `crypto`).
