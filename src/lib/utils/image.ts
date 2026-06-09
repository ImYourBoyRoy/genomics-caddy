// ./src/lib/utils/image.ts

/**
 * Loads an image file, resizes it using HTML5 Canvas to a maximum dimension of 1024px,
 * and converts it to a standard base64 JPEG format to prevent model context clutter.
 */
export function normalizeImage(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = (e) => {
      const img = new Image();
      img.onload = () => {
        const canvas = document.createElement("canvas");
        let width = img.width;
        let height = img.height;
        const maxDim = 1024;

        if (width > maxDim || height > maxDim) {
          if (width > height) {
            height = Math.round((height * maxDim) / width);
            width = maxDim;
          } else {
            width = Math.round((width * maxDim) / height);
            height = maxDim;
          }
        }

        canvas.width = width;
        canvas.height = height;
        const ctx = canvas.getContext("2d");
        if (!ctx) {
          reject(new Error("Failed to get canvas context"));
          return;
        }

        ctx.drawImage(img, 0, 0, width, height);
        const dataUrl = canvas.toDataURL("image/jpeg", 0.85);
        
        const base64Prefix = "data:image/jpeg;base64,";
        if (dataUrl.startsWith(base64Prefix)) {
          resolve(dataUrl.substring(base64Prefix.length));
        } else {
          const commaIndex = dataUrl.indexOf(",");
          if (commaIndex !== -1) {
            resolve(dataUrl.substring(commaIndex + 1));
          } else {
            resolve(dataUrl);
          }
        }
      };
      img.onerror = () => reject(new Error("Failed to load image element"));
      img.src = e.target?.result as string;
    };
    reader.onerror = () => reject(new Error("Failed to read file"));
    reader.readAsDataURL(file);
  });
}
