/** Choose a complete localized phrase, not just a singular/plural noun suffix. */
export function countMessage(
    count: number,
    zero: () => string,
    one: () => string,
    other: () => string,
): string {
    if (count === 0) return zero();
    if (count === 1) return one();
    return other();
}
