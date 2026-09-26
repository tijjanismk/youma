import { MessageCircle } from "lucide-react";
import { createPortal } from "react-dom";
import { lienWhatsApp, messageTicket } from "../whatsapp";

/**
 * Ticket pour l'impression par le navigateur (Ctrl+P, Win+P) : placé directement sous <body>, il est la seule chose
 * imprimée tant qu'il est présent (styles.css, `.zone-ticket`). Invisible à l'écran.
 */
export function TicketImprimable({ texte }: { texte: string }) {
  // Police réglée sur la plus longue ligne : le ticket tient dans les 72 mm utiles d'un rouleau de 80 mm, sans coupure.
  const colonnes = Math.max(24, ...texte.split("\n").map((l) => l.length));
  return createPortal(
    <div className="zone-ticket">
      <pre style={{ fontSize: `min(12pt, calc(72mm / ${(colonnes * 0.6).toFixed(1)}))` }}>{texte}</pre>
    </div>,
    document.body,
  );
}

/** Envoi du ticket par WhatsApp (lien wa.me), au numéro du client s'il est connu. */
export function TicketWhatsApp({ texte, telephone }: { texte: string; telephone?: string | null }) {
  return (
    <a className="bouton" href={lienWhatsApp(messageTicket(texte), telephone)} target="_blank" rel="noreferrer">
      <MessageCircle size={20} aria-hidden /> Envoyer par WhatsApp
    </a>
  );
}
