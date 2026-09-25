import type { Categorie } from "../types";

/** Teinte douce d'une couleur de catégorie (#rrggbb → fond du cercle). */
export function teinte(couleur: string, alpha = 0.16): string {
  const m = /^#([0-9a-f]{6})$/i.exec(couleur.trim());
  if (!m) return "#f1ece5";
  const n = parseInt(m[1], 16);
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${alpha})`;
}

/** Photo ronde du plat ; sans photo, l'icône de sa catégorie sur un cercle teinté (lisible hors ligne). */
export function VisuelPlat({ photo, categorie, petit }: { photo?: string | null; categorie?: Categorie; petit?: boolean }) {
  const classe = `visuel-plat ${petit ? "petit" : ""}`;
  if (photo) return <img className={classe} src={photo} alt="" loading="lazy" />;
  return (
    <span className={classe} style={{ background: teinte(categorie?.couleur ?? "") }} aria-hidden>
      {categorie?.icone || "🍽️"}
    </span>
  );
}

/** Réduit une image à `cote` px (carré recadré) en JPEG : stockée dans la base, sans serveur d'images. */
export function compresserImage(fichier: File, cote = 320): Promise<string> {
  return new Promise((resoudre, rejeter) => {
    const url = URL.createObjectURL(fichier);
    const img = new Image();
    img.onload = () => {
      const c = Math.min(img.width, img.height);
      const toile = document.createElement("canvas");
      toile.width = toile.height = cote;
      toile.getContext("2d")!.drawImage(img, (img.width - c) / 2, (img.height - c) / 2, c, c, 0, 0, cote, cote);
      URL.revokeObjectURL(url);
      resoudre(toile.toDataURL("image/jpeg", 0.75));
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      rejeter(new Error("Image illisible"));
    };
    img.src = url;
  });
}

/** Choix de la photo d'un plat : prise avec le téléphone ou fichier, compressée avant l'envoi. */
export function ChoixPhoto({ valeur, changer, categorie }: { valeur: string; changer: (v: string) => void; categorie?: Categorie }) {
  return (
    <div className="choix-photo">
      <VisuelPlat photo={valeur} categorie={categorie} />
      <label className="bouton">
        {valeur ? "Changer la photo" : "Ajouter une photo"}
        <input
          type="file"
          accept="image/*"
          hidden
          aria-label="Photo du plat"
          onChange={async (e) => {
            const f = e.target.files?.[0];
            if (f) changer(await compresserImage(f));
            e.target.value = "";
          }}
        />
      </label>
      {valeur && (
        <button type="button" onClick={() => changer("")}>
          Retirer
        </button>
      )}
    </div>
  );
}
