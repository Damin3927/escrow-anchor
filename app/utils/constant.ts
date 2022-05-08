import { expect } from "chai";

export const extractConstValue = (raw: string) => {
  expect(raw)
    .to.be.a("string")
    .and.match(/^b".*"$/);
  return raw.slice(2, raw.length - 1);
};
