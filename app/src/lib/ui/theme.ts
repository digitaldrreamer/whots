import type { Shape } from '$lib/game/types.js';

// Signature colour per shape — chosen for strong contrast against the ivory
// card face and to stay distinguishable for colour-blind players (varied
// hue + the shape itself always carries the meaning, colour is secondary).
export const SHAPE_COLORS: Record<Shape, string> = {
	circle: '#d64545',
	triangle: '#2f9e6f',
	cross: '#2f6fed',
	square: '#e0902a',
	star: '#8b5cf6'
};

export const SHAPE_LABELS: Record<Shape, string> = {
	circle: 'Circle',
	triangle: 'Triangle',
	cross: 'Cross',
	square: 'Square',
	star: 'Star'
};
