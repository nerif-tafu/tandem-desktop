import { ROOM_CODE_CHARSET, ROOM_CODE_LENGTH } from './constants.js';

export function normalizeRoomCode(input: string): string {
  return input.trim().toUpperCase();
}

export function isValidRoomCodeFormat(code: string): boolean {
  if (code.length !== ROOM_CODE_LENGTH) {
    return false;
  }

  for (const char of code) {
    if (!ROOM_CODE_CHARSET.includes(char as (typeof ROOM_CODE_CHARSET)[number])) {
      return false;
    }
  }

  return true;
}

export function generateRoomCode(random: () => number = Math.random): string {
  let code = '';

  for (let index = 0; index < ROOM_CODE_LENGTH; index += 1) {
    const charsetIndex = Math.floor(random() * ROOM_CODE_CHARSET.length);
    code += ROOM_CODE_CHARSET[charsetIndex];
  }

  return code;
}
