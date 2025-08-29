import { filterSharesFromCommitments } from "./examples/client/SecretSharingClient";
import { CryptoUtils } from "./examples/CryptoUtils";

/** Shares whose hash does not match its expected commitment are filtered away. */
test("filter shares from commitments", () => {
  const invalidShare = Buffer.from("invalidShare", "utf8");
  const validShare = Buffer.from("validShare", "utf8");

  const expectedCommitments = [
    CryptoUtils.hashBuffer(Buffer.from("correctFirstShare", "utf8")).toString("hex"),
    CryptoUtils.hashBuffer(Buffer.from("validShare", "utf8")).toString("hex"),
    CryptoUtils.hashBuffer(Buffer.from("missingShare", "utf8")).toString("hex"),
  ];

  const shares = [invalidShare, validShare, undefined];

  const filteredShares = filterSharesFromCommitments(expectedCommitments, shares);
  expect(filteredShares.length).toEqual(3);
  expect(filteredShares[0]).toEqual(undefined);
  expect(filteredShares[1]).toEqual(validShare);
  expect(filteredShares[2]).toEqual(undefined);
});
