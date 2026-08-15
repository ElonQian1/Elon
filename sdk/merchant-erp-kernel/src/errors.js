export class ErpKernelError extends Error {
  constructor(code, message, status = 400) {
    super(message);
    this.name = "ErpKernelError";
    this.code = code;
    this.status = status;
  }
}

export function fail(code, message, status = 400) {
  throw new ErpKernelError(code, message, status);
}
