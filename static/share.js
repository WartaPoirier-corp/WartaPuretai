const shareButton = document.querySelector("#share-icon");
shareButton.hidden = false; // By default, the button is hidden if there's no JavaScript
shareButton.addEventListener("click", share);

async function tryNavigatorShare(data) {
    if (typeof navigator.canShare === "function" && navigator.canShare(data)) {
        try {
            await navigator.share(data);
        } catch (e) {
            return !(e instanceof DOMException && e.type === "AbortError");
        }
    } else {
        return false;
    }
}

async function share() {
    const playerName = document.querySelector("#player-name").innerText;
    const [trashness, sex, alcohol, drugs] = [
        document.querySelector("#points-trashness").innerText,
        document.querySelector("#points-sex").innerText,
        document.querySelector("#points-alcohol").innerText,
        document.querySelector("#points-drugs").innerText,
    ];

    const shareObject = {
        title: "WartaPureté",
        text: `${playerName} a obtenu ce score à WartaPureté :
🗑️ Trashness : ${trashness} pts
💑 Sexe : ${sex} pts
🍺 Alcool : ${alcohol} pts
💊 Drogues : ${drugs} pts`,
        url: location.href,
    };

    if (await tryNavigatorShare(shareObject)) {
        // return
    } else if (navigator.clipboard && navigator.userActivation && navigator.userActivation.isActive) {
        await navigator.clipboard.writeText(location.href);
        alert("Lien copié !");
    } else {
        prompt("Copie ce lien:", location.href);
    }
}
