import reporter from "io-ts-reporters";
import { Errors } from "io-ts";
import { left } from "fp-ts/lib/Either";
import { Multiaddr } from "multiaddr";
export const sleep = (ms: number) =>
  new Promise((resolve) => setTimeout(resolve, ms));
export const zip = <A, B>(as: A[], bs: B[]): [A, B][] =>
  as.map((a, i) => [a, bs[i]]);

export const zipWithPromises = <A, B>(
  as: A[],
  promises: Promise<B>[]
): Promise<[A, B]>[] => as.map((a, i) => promises[i].then((b: B) => [a, b]));

export const ioErrToStr = (errs: Errors): string =>
  reporter.report(left(errs)).join(", ");

export const safeErrorToStr = (err: unknown): string => {
  if (!err) {
    return "none";
  }
  if (typeof err === "string") {
    return err;
  }

  if (typeof err === "object") {
    if (Object.prototype.hasOwnProperty.call(err, "shortMessage")) {
      if (Object.prototype.hasOwnProperty.call(err, "details")) {
        return (err as any).shortMessage + "(" + (err as any).details + ")";
      } else {
        return (err as any).shortMessage;
      }
    } else if (Object.prototype.hasOwnProperty.call(err, "message")) {
      return (err as any).message;
    }
  }
  return JSON.stringify(err, (_, v) =>
    typeof v === "function" ? "<func>" : v
  );
};

export const isValidMultiAddr = (str: string): boolean => {
  try {
    const m = new Multiaddr(str);
    return true;
  } catch (error) {
    return false;
  }
};

export const isValidMultiAddrWithPeerId = (str: string): boolean => {
  try {
    const m = new Multiaddr(str);
    return m.getPeerId() !== null;
  } catch (error) {
    return false;
  }
};

export const nodeAddrValid = (addr: string) =>
  !!/^((?:(?:(?:[a-zA-z-]+):\/{1,3})?(?:[a-zA-Z0-9])(?:[a-zA-Z0-9\-.]){1,61}(?:\.[a-zA-Z]{2,})+|\[(?:(?:(?:[a-fA-F0-9]){1,4})(?::(?:[a-fA-F0-9]){1,4}){7}|::1|::)\]|(?:(?:[0-9]{1,3})(?:\.[0-9]{1,3}){3}))(?::[0-9]{1,5})?)$|(localhost(?::[0-9]{1,5})?)$/.exec(
    addr
  );
