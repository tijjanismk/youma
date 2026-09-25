import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ChampMontant, PinPad } from "./composants/Base";
import { Fournisseur, useApp } from "./contexte";
import { post } from "./api";
import Connexion from "./pages/Connexion";

/** Faux poste central : réponses JSON par chemin. */
function fauxServeur(routes: Record<string, (corps: unknown, entetes: Record<string, string>) => [number, unknown]>) {
  const appels: { chemin: string; corps: unknown; entetes: Record<string, string> }[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      const chemin = url.replace(/^\/api/, "");
      const corps = init?.body ? JSON.parse(String(init.body)) : undefined;
      const entetes = (init?.headers ?? {}) as Record<string, string>;
      appels.push({ chemin, corps, entetes });
      const r = routes[chemin];
      if (!r) return new Response(JSON.stringify({ code: "NON_TROUVE", message: "?" }), { status: 404, headers: { "content-type": "application/json" } });
      const [statut, donnees] = r(corps, entetes);
      return new Response(JSON.stringify(donnees), { status: statut, headers: { "content-type": "application/json" } });
    }),
  );
  return appels;
}

const ETAT = {
  installe: true,
  demo: false,
  restaurant: "Maquis Test",
  horloge: { maintenant: 0, dernier_evenement: 0, coherente: true },
  journee: null,
  version: "0.1.0",
  reseau: false,
  parametres: {},
};

beforeEach(() => localStorage.clear());
afterEach(() => vi.unstubAllGlobals());

describe("PinPad", () => {
  it("se saisit au doigt et masque le code", async () => {
    const valider = vi.fn();
    render(<PinPad valider={valider} />);
    const u = userEvent.setup();
    const bouton = screen.getByRole("button", { name: "Valider" });
    expect(bouton).toBeDisabled();
    for (const c of "1234") await u.click(screen.getByRole("button", { name: `Chiffre ${c}` }));
    expect(screen.getByLabelText("PIN saisi")).toHaveTextContent("●●●●");
    expect(screen.queryByText("1234")).toBeNull();
    await u.click(bouton);
    expect(valider).toHaveBeenCalledWith("1234");
  });

  it("efface le dernier chiffre", async () => {
    const valider = vi.fn();
    render(<PinPad valider={valider} />);
    const u = userEvent.setup();
    for (const c of "12345") await u.click(screen.getByRole("button", { name: `Chiffre ${c}` }));
    await u.click(screen.getByRole("button", { name: "Effacer" }));
    await u.click(screen.getByRole("button", { name: "Valider" }));
    expect(valider).toHaveBeenCalledWith("1234");
  });
});

describe("ChampMontant", () => {
  it("n'accepte que des entiers et propose des raccourcis", async () => {
    const changer = vi.fn();
    render(<ChampMontant libelle="Montant" valeur={0} changer={changer} raccourcis={[5000, 10000]} />);
    const u = userEvent.setup();
    await u.type(screen.getByLabelText("Montant"), "7");
    expect(changer).toHaveBeenLastCalledWith(7);
    await u.click(screen.getByRole("button", { name: "10 000" }));
    expect(changer).toHaveBeenLastCalledWith(10000);
  });
});

describe("Connexion", () => {
  it("affiche les utilisateurs puis connecte avec le PIN", async () => {
    const appels = fauxServeur({
      "/etat": () => [200, ETAT],
      "/connexion/utilisateurs": () => [200, [{ id: "u1", nom: "Awa", role: "Serveur" }]],
      "/connexion": (c) =>
        (c as { pin: string }).pin === "4444"
          ? [200, { jeton: "J", utilisateur: { id: "u1", nom: "Awa", role_code: "serveur", role_nom: "Serveur", employe_id: null }, permissions: [], plafond_remise_pct: 0 }]
          : [401, { code: "PIN_INCORRECT", message: "Code PIN incorrect" }],
    });
    vi.stubGlobal("WebSocket", class { close() {} } as unknown as typeof WebSocket);
    render(
      <Fournisseur>
        <MemoryRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
          <Connexion />
        </MemoryRouter>
      </Fournisseur>,
    );
    const u = userEvent.setup();
    await u.click(await screen.findByRole("button", { name: /Awa/ }));
    for (const c of "0000") await u.click(screen.getByRole("button", { name: `Chiffre ${c}` }));
    await u.click(screen.getByRole("button", { name: "Entrer" }));
    expect(await screen.findByText("Code PIN incorrect")).toBeInTheDocument();
    for (const c of "4444") await u.click(screen.getByRole("button", { name: `Chiffre ${c}` }));
    await u.click(screen.getByRole("button", { name: "Entrer" }));
    await waitFor(() => expect(localStorage.getItem("youma.jeton")).toBe("J"));
    expect(appels.filter((a) => a.chemin === "/connexion")).toHaveLength(2);
  });
});

function BoutonAction() {
  const { agir } = useApp();
  return <button onClick={() => agir((pin) => post("/lignes/l1/annuler", { quantite: 1, motif: "Erreur" }, pin), "Article annulé")}>Annuler l'article</button>;
}

describe("Autorisation ponctuelle (RG-AUT-03)", () => {
  it("demande le PIN d'un responsable puis rejoue l'action avec ce PIN", async () => {
    const appels = fauxServeur({
      "/etat": () => [200, ETAT],
      "/lignes/l1/annuler": (_c, entetes) =>
        entetes["X-Autorisation-Pin"] === "2222"
          ? [200, null]
          : [403, { code: "AUTORISATION_REQUISE", message: "Autorisation d'un responsable requise", permission: "commande.annuler_envoye" }],
    });
    render(
      <Fournisseur>
        <BoutonAction />
      </Fournisseur>,
    );
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Annuler l'article" }));
    const dialogue = await screen.findByRole("dialog", { name: "PIN du responsable" });
    expect(dialogue).toBeInTheDocument();
    for (const c of "2222") await u.click(screen.getByRole("button", { name: `Chiffre ${c}` }));
    await u.click(screen.getByRole("button", { name: "Autoriser" }));
    expect(await screen.findByText("Article annulé")).toBeInTheDocument();
    const tentatives = appels.filter((a) => a.chemin === "/lignes/l1/annuler");
    expect(tentatives).toHaveLength(2);
    expect(tentatives[1].entetes["X-Autorisation-Pin"]).toBe("2222");
  });

  it("signale le poste central injoignable sans perdre l'action", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        throw new TypeError("Failed to fetch");
      }),
    );
    render(
      <Fournisseur>
        <BoutonAction />
      </Fournisseur>,
    );
    const u = userEvent.setup();
    await u.click(screen.getByRole("button", { name: "Annuler l'article" }));
    expect(await screen.findByText(/Poste central injoignable/)).toBeInTheDocument();
  });
});

describe("espace propriétaire (fiche 0018)", () => {
  it("connexion puis résumé de chaque restaurant et total", async () => {
    const resume = (date: string, ca: number) => ({
      date,
      cloturee: true,
      chiffre_affaires: ca,
      commandes: 10,
      depenses: 0,
      encaissements: [["especes", ca]],
      mobile_money_a_verifier: [1, 2_000],
      annulations: [0, 0],
      ecarts_caisse: 0,
      mis_a_jour: 0,
    });
    const appels = fauxServeur({
      "/proprietaire/connexion": (c) => ((c as { mot_de_passe: string }).mot_de_passe === "bon-mot-de-passe" ? [200, { jeton: "J1", restaurants: 2 }] : [401, { code: "NON_AUTHENTIFIE", message: "Numéro ou mot de passe incorrect" }]),
      "/proprietaire/tableau": () => [
        200,
        {
          restaurants: [
            { id: "a", nom: "Maquis A", dernier_contact: 0, derniere_sauvegarde: null, resumes: [resume("2026-03-14", 125_000)] },
            { id: "b", nom: "Maquis B", dernier_contact: 0, derniere_sauvegarde: 0, resumes: [resume("2026-03-14", 75_000)] },
          ],
          totaux: [{ date: "2026-03-14", chiffre_affaires: 200_000, commandes: 20 }],
        },
      ],
    });
    const { default: Proprietaire } = await import("./public/Proprietaire");
    render(<Proprietaire />);
    const u = userEvent.setup();
    await u.type(screen.getByLabelText("Votre téléphone"), "76 00 00 01");
    await u.type(screen.getByLabelText("Mot de passe distant"), "mauvais");
    await u.click(screen.getByRole("button", { name: "Se connecter" }));
    expect(await screen.findByText("Numéro ou mot de passe incorrect")).toBeInTheDocument();
    await u.clear(screen.getByLabelText("Mot de passe distant"));
    await u.type(screen.getByLabelText("Mot de passe distant"), "bon-mot-de-passe");
    await u.click(screen.getByRole("button", { name: "Se connecter" }));
    expect(await screen.findByRole("heading", { name: "Maquis B" })).toBeInTheDocument();
    expect(screen.getByLabelText("Tous les restaurants")).toHaveTextContent("200 000 FCFA");
    expect(screen.getByLabelText("Maquis A")).toHaveTextContent("Mobile Money à vérifier : 1 (2 000 FCFA)");
    expect(appels.find((a) => a.chemin === "/proprietaire/tableau")?.entetes.Authorization).toBe("Bearer J1");
  });
});
