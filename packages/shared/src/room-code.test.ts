import { describe, expect, it } from 'vitest';

import { generateRoomCode, isValidRoomCodeFormat, normalizeRoomCode } from './room-code.js';

describe('room-code', () => {
  it('generates 5-character codes from allowed charset', () => {
    const code = generateRoomCode(() => 0);
    expect(code).toHaveLength(5);
    expect(isValidRoomCodeFormat(code)).toBe(true);
  });

  it('rejects ambiguous and invalid characters', () => {
    expect(isValidRoomCodeFormat('O0123')).toBe(false);
    expect(isValidRoomCodeFormat('ABCDE')).toBe(true);
  });

  it('normalizes room codes to uppercase', () => {
    expect(normalizeRoomCode(' abc23 ')).toBe('ABC23');
  });
});
