function run() {
    const dia = Application("company.thebrowser.dia");
    if (!dia.running()) throw new Error("Open Dia first");
    const windows = dia.windows();
    if (windows.length === 0) throw new Error("Open a Dia window first");
    return JSON.stringify(windows[0].profiles().map(profile => profile.name()));
}
