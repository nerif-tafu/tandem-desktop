export class TandemError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = this.constructor.name;
    this.code = code;
  }
}

export class RoomNotFoundError extends TandemError {
  constructor(roomCode: string) {
    super('ROOM_NOT_FOUND', `Room ${roomCode} was not found or has expired`);
  }
}

export class InvalidRoomCodeError extends TandemError {
  constructor() {
    super('INVALID_ROOM_CODE', 'Room code must be 5 valid characters');
  }
}

export class RoomCodeTakenError extends TandemError {
  constructor(code: string) {
    super('ROOM_CODE_TAKEN', `Room code ${code} is already in use`);
  }
}

export class InvalidRoomPasswordError extends TandemError {
  constructor() {
    super('INVALID_ROOM_PASSWORD', 'Incorrect room password');
  }
}

export class RoomPasswordRequiredError extends TandemError {
  constructor() {
    super('ROOM_PASSWORD_REQUIRED', 'This room requires a password');
  }
}

export class CaptureInitError extends TandemError {
  constructor(message: string) {
    super('CAPTURE_INIT_ERROR', message);
  }
}
