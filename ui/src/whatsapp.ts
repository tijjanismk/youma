/**
 * WhatsApp par liens « wa.me » (fiche 0028) : ouvre WhatsApp sur l'appareil avec le message prêt, l'utilisateur
 * appuie sur « Envoyer ». Aucune API ni compte WhatsApp Business : les textes viennent du poste central, une
 * future API WhatsApp pourra envoyer les mêmes messages depuis le serveur.
 */

/** Numéro malien au format international sans « + » (70 00 00 00 → 22370000000) ; vide si inutilisable. */
export function numeroWhatsApp(telephone?: string | null): string {
  let n = (telephone ?? "").replace(/\D/g, "");
  if (n.startsWith("00")) n = n.slice(2);
  if (n.length === 8) n = `223${n}`;
  return n.length >= 10 ? n : "";
}

/** Lien wa.me ; sans numéro, WhatsApp demande à qui envoyer. */
export function lienWhatsApp(texte: string, telephone?: string | null): string {
  return `https://wa.me/${numeroWhatsApp(telephone)}?text=${encodeURIComponent(texte)}`;
}

/** Ticket en police à chasse fixe dans WhatsApp (les colonnes restent alignées). */
export function messageTicket(ticket: string): string {
  return "```\n" + ticket.trim() + "\n```";
}
