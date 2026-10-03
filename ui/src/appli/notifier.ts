// Application Youma Client (fiche 0041) : notification quand la commande change d'étape, tant que l'application
// tourne (au premier plan ou en arrière-plan récent). Pas de serveur de notifications : rien ne part sur Internet.
let autorise: boolean | null = null;

export async function notifier(titre: string, texte: string) {
  try {
    const { LocalNotifications } = await import("@capacitor/local-notifications");
    if (autorise === null) autorise = (await LocalNotifications.requestPermissions()).display === "granted";
    if (!autorise) return;
    await LocalNotifications.schedule({ notifications: [{ id: Date.now() % 2_000_000_000, title: titre, body: texte }] });
  } catch {
    /* navigateur ou refus : le suivi à l'écran suffit */
  }
}
