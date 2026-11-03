/** Luminância relativa de uma cor `#RRGGBB` ou `#RGB` (WCAG 2.x). */
function luminance(hex: string): number {
  let digits = hex.replace("#", "");
  if (digits.length === 3) digits = [...digits].map((c) => c + c).join("");
  const channels = [0, 2, 4].map((i) => {
    const value = parseInt(digits.slice(i, i + 2), 16) / 255;
    return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}

/** Razão de contraste entre duas cores (1 a 21); texto normal exige pelo menos 4,5. */
export function contrastRatio(foreground: string, background: string): number {
  const a = luminance(foreground);
  const b = luminance(background);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}
