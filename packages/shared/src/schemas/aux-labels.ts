import { z } from 'zod';

export const AuxSlotLabelsSchema = z.object({
  aux1: z.string().min(1).max(64).optional(),
  aux2: z.string().min(1).max(64).optional(),
});
