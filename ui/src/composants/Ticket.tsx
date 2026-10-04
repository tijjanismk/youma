import { MessageCircle } from "lucide-react";
import { createPortal } from "react-dom";
import { useApp } from "../contexte";
import { lienWhatsApp, messageTicket } from "../whatsapp";

/**
 * Ticket pour l'impression par le navigateur (Ctrl+P, Win+P) : placé directement sous <body>, il est la seule chose
 * imprimée tant qu'il est présent (styles.css, `.zone-ticket`). Invisible à l'écran.
 */
export function TicketImprimable({ texte }: { texte: string }) {
  const { etat } = useApp();
  // Police réglée sur la plus longue ligne : le ticket tient dans la largeur utile du rouleau, sans coupure.
  const colonnes = Math.max(24, ...texte.split("\n").map((l) => l.length));
  const largeur = largeurImprimableMm(etat?.parametres?.largeur_ticket ?? 42);
  return createPortal(
    <div className="zone-ticket">
      <pre style={{ width: `${largeur}mm`, fontSize: `min(12pt, calc(${largeur}mm / ${(colonnes * 0.6).toFixed(1)}))` }}>{texte}</pre>
    </div>,
    document.body,
  );
}

/** Largeur utile (mm) selon le papier réglé : 50 mm → 40, 58 mm → 48, 80 mm → 72. */
export function largeurImprimableMm(caracteres: number): number {
  if (caracteres <= 28) return 40;
  if (caracteres <= 32) return 48;
  return 72;
}

/** Envoi du ticket par WhatsApp (lien wa.me), au numéro du client s'il est connu. */
export function TicketWhatsApp({ texte, telephone }: { texte: string; telephone?: string | null }) {
  return (
    <a className="bouton" href={lienWhatsApp(messageTicket(texte), telephone)} target="_blank" rel="noreferrer">
      <MessageCircle size={20} aria-hidden /> Envoyer par WhatsApp
    </a>
  );
}
