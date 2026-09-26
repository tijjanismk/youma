import { useApp } from "../contexte";
import type { Categorie } from "../types";

/** Teinte douce d'une couleur de catégorie (#rrggbb → fond du cercle). */
export function teinte(couleur: string, alpha = 0.16): string {
  const m = /^#([0-9a-f]{6})$/i.exec(couleur.trim());
  if (!m) return "var(--carte-2)";
  const n = parseInt(m[1], 16);
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${alpha})`;
}

/**
 * Visuel du plat : la photo entière (jamais rognée, quelle que soit sa forme) dans un cadre arrondi ;
 * sans photo, l'icône de sa catégorie sur un cercle teinté (lisible hors ligne).
 */
export function VisuelPlat({ photo, categorie, petit }: { photo?: string | null; categorie?: Pick<Categorie, "icone" | "couleur">; petit?: boolean }) {
  const classe = `visuel-plat ${petit ? "petit" : ""}`;
  if (photo) return <img className={`${classe} photo`} src={photo} alt="" loading="lazy" />;
  return (
    <span className={classe} style={{ background: teinte(categorie?.couleur ?? "") }} aria-hidden>
      {categorie?.icone || "🍽️"}
    </span>
  );
}

/**
 * Réduit une image à `cote` px sur son plus grand côté, **sans la recadrer** (photo en largeur, en hauteur ou
 * carrée), en JPEG : stockée dans la base, sans serveur d'images. Fond blanc sous les PNG transparents.
 */
export function compresserImage(fichier: File, cote = 480): Promise<string> {
  return new Promise((resoudre, rejeter) => {
    const url = URL.createObjectURL(fichier);
    const img = new Image();
    img.onload = () => {
      const echelle = Math.min(1, cote / Math.max(img.width, img.height));
      const toile = document.createElement("canvas");
      toile.width = Math.max(1, Math.round(img.width * echelle));
      toile.height = Math.max(1, Math.round(img.height * echelle));
      const ctx = toile.getContext("2d")!;
      ctx.fillStyle = "#fff";
      ctx.fillRect(0, 0, toile.width, toile.height);
      ctx.drawImage(img, 0, 0, toile.width, toile.height);
      URL.revokeObjectURL(url);
      resoudre(toile.toDataURL("image/jpeg", 0.8));
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      rejeter(new Error("Image illisible : choisissez une photo JPEG, PNG ou WebP"));
    };
    img.src = url;
  });
}

/** Choix de la photo d'un plat : prise avec le téléphone ou fichier, compressée avant l'envoi. */
export function ChoixPhoto({ valeur, changer, categorie }: { valeur: string; changer: (v: string) => void; categorie?: Categorie }) {
  const { notifier } = useApp();
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
            if (f)
              await compresserImage(f)
                .then(changer)
                .catch((err: Error) => notifier(err.message, "erreur"));
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
