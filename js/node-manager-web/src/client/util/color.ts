export const HslPercentageSpectrum =
  (hueAt0: number, hueAt1: number, saturation?: number, luminosity?: number) =>
  (percentage: number) =>
    `hsl(${(percentage / 100) * (hueAt1 - hueAt0) + hueAt0},${
      saturation || 100
    }%,${luminosity || 100}%)`;

export const RedToGreenPercentageSpectrum = HslPercentageSpectrum(0, 120);
