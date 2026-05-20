package main

import (
	"context"
	"fmt"
	"io"
	"log"
	"log/slog"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"slices"
	"strings"
	"time"

	"github.com/cedws/w101-client-go/codegen"
	"github.com/cedws/w101-client-go/proto"
	"github.com/cedws/w101-client-go/wad"
	"github.com/cedws/w101-proto-go/pkg/patch"
)

const patchServer = "patch.us.wizard101.com:12500"

var errTimeoutFileList = fmt.Errorf("timed out waiting for latest file list")

var protocolFiles = []string{
	"AISClientMessages.xml",
	"BaseMessages.xml",
	"ExtendedBaseMessages.xml",
	"GameMessages.xml",
	"LoginMessages.xml",
	"PatchMessages.xml",
	"PetMessages.xml",
	"ScriptDebuggerMessages.xml",
	"TestManagerMessages.xml",
	"WizardMessages.xml",
	"Messages/CantripsMessages.xml",
	"Messages/CatchAKeyMessages.xml",
	"Messages/ChooChooZooMessages.xml",
	"Messages/ConcentrationMessages.xml",
	"Messages/DoodleDougMessages.xml",
	"Messages/Dueling_DiegoMessages.xml",
	"Messages/HotShotsMessages.xml",
	"Messages/HousingMessages.xml",
	// "Messages/MoveBehaviorMessages.xml",
	// "Messages/PhysicsBehaviorMessages.xml",
	"Messages/PotionMotionMessages.xml",
	"Messages/QuestMessages.xml",
	"Messages/ShockALockMessages.xml",
	"Messages/SkullRidersMessages.xml",
	"Messages/SoblocksMessages.xml",
	"Messages/WizCombatMessages.xml",
}

type patchHandler struct {
	patch.Service
	fileListCh chan patch.LatestFileListV2
}

func (p patchHandler) LatestFileListV2(m patch.LatestFileListV2) {
	p.fileListCh <- m
}

func main() {
	if err := generateProtocolFiles(context.Background()); err != nil {
		log.Fatal(err)
	}

	slog.Info("Downloaded message files")
}

func generateProtocolFiles(ctx context.Context) error {
	if err := downloadRootWAD(ctx); err != nil {
		return err
	}

	wadArchive, err := wad.Open("Root.wad")
	if err != nil {
		return err
	}
	defer wadArchive.Close()

	protocols, err := extractProtocols(wadArchive)
	if err != nil {
		return err
	}

	for _, protocol := range protocols {
		var (
			name     = strings.ToLower(protocol.Service)
			dirname  = filepath.Join("pkg", name)
			filename = filepath.Join(dirname, name+".go")
		)

		if err := os.MkdirAll(dirname, 0o755); err != nil {
			return err
		}

		goFile, err := os.Create(filename)
		if err != nil {
			return err
		}

		slog.Info("Generating protocol file", "path", filename)
		if err := codegen.Generate(goFile, name, protocol); err != nil {
			return err
		}
	}

	return nil
}

func extractProtocols(wadArchive *wad.Archive) ([]codegen.Protocol, error) {
	protocols := make([]codegen.Protocol, 0)

	for entry := range wadArchive.Entries() {
		if !slices.Contains(protocolFiles, entry.Path) {
			continue
		}

		slog.Info("Extracting protocols file", "path", entry.Path)

		entryReader, err := wadArchive.Entry(entry)
		if err != nil {
			return nil, err
		}

		protoBytes, err := io.ReadAll(entryReader)
		if err != nil {
			return nil, err
		}

		proto, err := codegen.UnmarshalProtocol(protoBytes)
		if err != nil {
			return nil, err
		}

		protocols = append(protocols, proto)
	}

	return protocols, nil
}

func downloadRootWAD(ctx context.Context) error {
	wadReader, err := latestRootWAD(ctx)
	if err != nil {
		return err
	}
	defer wadReader.Close()

	wadFile, err := os.Create("Root.wad")
	if err != nil {
		return err
	}
	defer wadFile.Close()

	slog.Info("Downloading Root.wad")

	if _, err := io.Copy(wadFile, wadReader); err != nil {
		return err
	}

	return nil
}

func latestRootWAD(ctx context.Context) (io.ReadCloser, error) {
	fileListCh := make(chan patch.LatestFileListV2)

	r := proto.NewMessageRouter()
	patch.RegisterService(&r, &patchHandler{fileListCh: fileListCh})

	dialCtx, cancel := context.WithTimeoutCause(ctx, 10*time.Second, errTimeoutFileList)
	defer cancel()

	client, err := proto.Dial(dialCtx, patchServer, &r)
	if err != nil {
		return nil, err
	}
	defer client.Close()

	slog.Info("Connected to patch server", "server", patchServer)

	c := patch.NewClient(client)
	if err := c.LatestFileListV2(&patch.LatestFileListV2{}); err != nil {
		return nil, err
	}

	select {
	case fileList := <-fileListCh:
		slog.Info("Received latest file list", "url", fileList.ListFileURL)
		return requestRootWAD(ctx, fileList)
	case <-ctx.Done():
		return nil, ctx.Err()
	}
}

func requestRootWAD(ctx context.Context, fileList patch.LatestFileListV2) (io.ReadCloser, error) {
	wadURL, err := url.JoinPath(fileList.URLPrefix, "Data/GameData/Root.wad")
	if err != nil {
		return nil, err
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, wadURL, http.NoBody)
	if err != nil {
		return nil, err
	}
	req.Header.Set("User-Agent", "KingsIsle Patcher")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return nil, err
	}

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("unexpected status code: %d", resp.StatusCode)
	}

	return resp.Body, err
}
