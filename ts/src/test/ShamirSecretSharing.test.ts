import { ShamirFactory, ShamirSecretShares } from "./examples/secretsharing/ShamirSecretShares";
import { F256 } from "./examples/secretsharing/F256";

const FACTORY = ShamirSecretShares.FACTORY;

/** Generated shares from a plaintext can be reconstructed into the same plaintext. */
test("secret share", () => {
  const secret = "hello world";
  const secretBytes = Buffer.from(secret, "utf8");

  const secretShares = FACTORY.fromPlainText(4, secretBytes);

  const shares = [
    secretShares.getShareBytes(0),
    secretShares.getShareBytes(1),
    secretShares.getShareBytes(2),
    secretShares.getShareBytes(3),
  ];

  const readShares = FACTORY.fromSharesBytes(shares);
  const reconstructedSecretBytes = readShares.reconstructPlainText();
  const reconstructedSecret = reconstructedSecretBytes.toString("utf-8");

  expect(reconstructedSecret).toEqual(secret);
});

/** Only two out of four shares are required to reconstruct the shares. */
test("secret share missing shares", () => {
  const secret = "hello world";
  const secretBytes = Buffer.from(secret, "utf8");

  const secretShares = FACTORY.fromPlainText(4, secretBytes);

  const shares: Array<Buffer | undefined> = [
    secretShares.getShareBytes(0),
    undefined,
    secretShares.getShareBytes(2),
    undefined,
  ];

  const readShares = FACTORY.fromSharesBytes(shares);
  const reconstructedSecretBytes = readShares.reconstructPlainText();
  const reconstructedSecret = reconstructedSecretBytes.toString("utf-8");

  expect(reconstructedSecret).toEqual(secret);
});

/**
 * Test that plain-text can be reconstructed from {@link ShamirSecretShares} with a few enough
 * incorrect shares.
 */
test("secret share incorrect shares", () => {
  const secret = "hello world";
  const secretBytes = Buffer.from(secret, "utf8");

  const factory = new ShamirFactory({ numMalicious: 2, numNodes: 7, numToReconstruct: 5 });
  const secretShares = factory.fromPlainText(7, secretBytes);

  const shares: Array<Buffer | undefined> = [
    secretShares.getShareBytes(0),
    Buffer.alloc(secretBytes.length),
    secretShares.getShareBytes(2),
    secretShares.getShareBytes(3),
    Buffer.alloc(secretBytes.length),
    secretShares.getShareBytes(5),
    secretShares.getShareBytes(6),
  ];

  const readShares = factory.fromSharesBytes(shares);
  const reconstructedSecretBytes = readShares.reconstructPlainText();
  const reconstructedSecret = reconstructedSecretBytes.toString("utf-8");

  expect(reconstructedSecret).toEqual(secret);
});

/**
 * Unable to reconstruct the plain text from {@link ShamirSecretShares} with too many incorrect
 * shares.
 */
test("can't reconstruct secret share enough incorrect shares", () => {
  const secret = "hello world";
  const secretBytes = Buffer.from(secret, "utf8");

  const factory = new ShamirFactory({ numMalicious: 2, numNodes: 7, numToReconstruct: 5 });
  const secretShares = factory.fromPlainText(7, secretBytes);

  const shares: Array<Buffer | undefined> = [
    secretShares.getShareBytes(0),
    Buffer.alloc(secretBytes.length),
    secretShares.getShareBytes(2),
    Buffer.alloc(secretBytes.length),
    Buffer.alloc(secretBytes.length),
    secretShares.getShareBytes(5),
    secretShares.getShareBytes(6),
  ];

  const readShares = factory.fromSharesBytes(shares);
  expect(() => readShares.reconstructPlainText()).toThrow("Unable to reconstruct secret");
});

/** Multiplication for F256 is associative and commutative. */
test("F256 mul", () => {
  const a = F256.createElement(7);
  const b = F256.createElement(167);
  const c = F256.createElement(54);

  const x = a.multiply(b).multiply(c);
  const y = a.multiply(b.multiply(c));

  expect(x).toEqual(y);
  expect(a.multiply(b)).toEqual(b.multiply(a));
});

/** F256 has a multiplicative inverse with one as the identity. */
test("F256 inverse", () => {
  for (let i = 1; i < 256; i++) {
    const a = F256.createElement(i);
    const b = a.modInverse();

    expect(b.multiply(a)).toEqual(F256.ONE);
  }
});
