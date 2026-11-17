export type FloorPoint = { x: number; y: number };

export type FloorImage = {
  jpeg: string;
  pixel_width: number;
  pixel_height: number;
};

export type WifiFloor = FloorImage & {
  from: FloorPoint;
  to: FloorPoint;
  distance_cm: number;
};

export type FloorUpdate =
  | { kind: "set"; floor: WifiFloor }
  | { kind: "remove" };

export async function prepareFloor(file: File): Promise<FloorImage> {
  if (!["image/png", "image/jpeg", "image/webp"].includes(file.type)) {
    throw new Error("请选择 JPG、PNG 或 WebP 图片。");
  }
  if (file.size > 12 * 1024 * 1024)
    throw new Error("请选择 12 MB 以内的图片。");
  const bitmap = await createImageBitmap(file).catch(() => {
    throw new Error("无法读取该图片，请换一张底图。");
  });
  try {
    const surface = document.createElement("canvas");
    const context = surface.getContext("2d");
    if (!context) throw new Error("当前浏览器无法处理图片。");
    for (const maxEdge of [1600, 1280, 960, 720]) {
      const scale = Math.min(
        1,
        maxEdge / Math.max(bitmap.width, bitmap.height),
      );
      surface.width = Math.max(1, Math.round(bitmap.width * scale));
      surface.height = Math.max(1, Math.round(bitmap.height * scale));
      context.fillStyle = "#fff";
      context.fillRect(0, 0, surface.width, surface.height);
      context.drawImage(bitmap, 0, 0, surface.width, surface.height);
      for (const quality of [0.88, 0.74, 0.6]) {
        const jpeg = surface.toDataURL("image/jpeg", quality).split(",")[1];
        if (jpeg && jpeg.length <= 180_000) {
          return {
            jpeg,
            pixel_width: surface.width,
            pixel_height: surface.height,
          };
        }
      }
    }
    throw new Error("底图压缩后仍过大，请裁剪或换图。");
  } finally {
    bitmap.close();
  }
}

export function calibratedSize(
  image: FloorImage,
  from: FloorPoint,
  to: FloorPoint,
  distanceCm: number,
) {
  if (distanceCm < 10 || distanceCm > 7071) return;
  const dx = ((to.x - from.x) * image.pixel_width) / 1000;
  const dy = ((to.y - from.y) * image.pixel_height) / 1000;
  const pixels = Math.hypot(dx, dy);
  if (pixels < 5) return;
  const width = Math.round((image.pixel_width * distanceCm) / pixels);
  const height = Math.round((image.pixel_height * distanceCm) / pixels);
  if ([width, height].every((value) => value >= 100 && value <= 5000))
    return { width, height };
}
