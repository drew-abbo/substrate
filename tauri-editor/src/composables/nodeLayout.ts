export const HEADER_H = 30
export const ROW_H    = 26

/** Top position for an output handle (right side). */
export function outputHandleTop(outputIndex: number): string {
  return `${HEADER_H + outputIndex * ROW_H + ROW_H / 2}px`
}

/**
 * Top position for an input handle (left side).
 * Accounts for the output rows rendered above the input section.
 */
export function inputHandleTop(inputIndex: number, outputCount: number): string {
  return `${HEADER_H + outputCount * ROW_H + inputIndex * ROW_H + ROW_H / 2}px`
}
