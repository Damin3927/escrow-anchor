import { AssertionError, expect } from "chai";
import { extractConstValue } from "../../app/utils/constant";

describe("constant", function () {
  describe("extractConstValue", function () {
    context("when the format is valid", function () {
      it("extracts the value", () => {
        expect(extractConstValue('b"hoge"')).to.equal("hoge");
      });
    });

    context("when the format is invalid", function () {
      it("raises an error when the value is null", function () {
        expect(() => extractConstValue(null)).to.throw(AssertionError);
      });

      it("raises an error when the value is empty", function () {
        expect(() => extractConstValue("")).to.throw(AssertionError);
      });

      it("raises an error when the value has an invalid format", function () {
        expect(() => extractConstValue('b"hoge')).to.throw(AssertionError);
      });
    });
  });
});
